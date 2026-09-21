#!/usr/bin/env python3
"""Generate real themes in isolation and verify formats plus native editor loading."""
import argparse
import configparser
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[4]
TARGETS = ('claude', 'codex', 'herdr', 'hyprtoolkit', 'kvantum', 'limine', 'neovim', 'steam', 'vim')
HEX = re.compile(r'#[0-9a-fA-F]{6}\Z')


def run(args, env):
    result = subprocess.run(args, env=env, text=True, capture_output=True, timeout=60)
    if result.returncode:
        raise RuntimeError(f'{args!r}\n{result.stdout}\n{result.stderr}')
    return result.stdout


def verify(binary, directory):
    env = dict(os.environ)
    for key in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME'):
        env[key] = str(directory / key.lower())
        Path(env[key]).mkdir(parents=True, exist_ok=True)
    for mode in ('dark', 'light'):
        for target in TARGETS:
            out = directory / mode / target
            run([str(binary), 'generate', '--seed', '#4ecdc4', '--template', 'materialish',
                 '--mode', mode, '--targets', target, '--output', str(out), '--force'], env)
            assert all('{{' not in p.read_text() for p in out.iterdir()), target
        out = directory / mode
        claude = json.loads((out / 'claude/chromasync.json').read_text())
        assert claude['base'] == mode
        assert all(HEX.fullmatch(v) for v in claude['overrides'].values())
        codex = plistlib.loads((out / 'codex/chromasync.tmTheme').read_bytes())
        assert codex['name'] == 'Chromasync'
        settings = codex['settings']
        assert all(HEX.fullmatch(v) for rule in settings for v in rule['settings'].values())
        palette = settings[0]['settings']
        assert any('keyword' in x.get('scope', '') for x in settings)
        assert any('markup.deleted' in x.get('scope', '') for x in settings)
        herdr = tomllib.loads((out / 'herdr/chromasync-herdr.toml').read_text())
        assert set(herdr) == {'theme'}
        assert herdr['theme']['auto_switch'] is False
        assert len(herdr['theme']['custom']) == 19
        assert all(HEX.fullmatch(v) for v in herdr['theme']['custom'].values())
        hypr = dict(line.split(' = ') for line in (out / 'hyprtoolkit/chromasync-hyprtoolkit.conf').read_text().splitlines() if line and not line.startswith('#'))
        assert len(hypr) == 8
        assert all(re.fullmatch(r'0xff[0-9a-f]{6}', value) for value in hypr.values())
        assert hypr['background'] != hypr['text']
        limine = dict(line.split(': ') for line in (out / 'limine/chromasync-limine.conf').read_text().splitlines() if line and not line.startswith('#'))
        for name in ('term_palette', 'term_palette_bright'):
            assert len(limine[name].split(';')) == 8
            assert all(re.fullmatch(r'[0-9a-f]{6}', c) for c in limine[name].split(';'))
        assert re.fullmatch(r'00[0-9a-f]{6}', limine['term_background'])
        kv = configparser.ConfigParser(interpolation=None)
        kv.read(out / 'kvantum/chromasync.kvconfig')
        assert kv['GeneralColors']['highlight.text.color'] == kv['GeneralColors']['text.color']
        svg = ET.parse(out / 'kvantum/chromasync.svg')
        elements = [e.attrib['id'] for e in svg.iter() if 'id' in e.attrib]
        assert len(elements) == len(set(elements)), 'duplicate SVG IDs'
        for section in kv.values():
            if 'interior.element' in section:
                assert section['interior.element'] + '-normal' in elements
        for role in ('checkbox', 'radio'):
            assert f'cs-{role}-checked-normal' in elements
        assert 'cs-focus-top' in elements
        for role in ('button', 'input'):
            assert f'cs-{role}-normal-left' in elements
        steam = json.loads((out / 'steam/skin.json').read_text())
        for patch in steam['Patches']:
            assert (out / 'steam' / patch['TargetCss']).is_file()
            assert 'TargetJs' not in patch
            assert re.search(patch['MatchRegexString'], 'Steam')
        # Native editors must load their generated colorschemes, not only parse text.
        vim_test = directory / 'check.vim'
        vim_test.write_text('\n'.join([
            'set termguicolors',
            f'source {out / "vim/chromasync.vim"}',
            'call assert_equal("chromasync", g:colors_name)',
            f'call assert_equal("{mode}", &background)',
            f'call assert_equal("{palette["foreground"].lower()}", tolower(synIDattr(hlID("Normal"), "fg#")))',
            f'call assert_equal("{palette["background"].lower()}", tolower(synIDattr(hlID("Normal"), "bg#")))',
            'call assert_equal(16, len(g:terminal_ansi_colors))',
            'if len(v:errors) | cquit | endif', 'qa!',
        ]))
        run(['vim', '-Nu', 'NONE', '-i', 'NONE', '-n', '-es', '-S', str(vim_test)], env)
        lua_test = directory / 'check.lua'
        lua_test.write_text('\n'.join([
            'vim.opt.termguicolors = true',
            f'dofile({json.dumps(str(out / "neovim/chromasync.lua"))})',
            'assert(vim.g.colors_name == "chromasync")',
            f'assert(vim.o.background == "{mode}")',
            'local normal = vim.api.nvim_get_hl(0, { name = "Normal", link = false })',
            f'assert(normal.fg == {int(palette["foreground"][1:], 16)})',
            f'assert(normal.bg == {int(palette["background"][1:], 16)})',
            'local capture = vim.api.nvim_get_hl(0, { name = "@function", link = true })',
            'assert(capture.link == "Function")',
            'assert(vim.api.nvim_get_hl(0, { name = "DiagnosticUnderlineError" }).undercurl)',
            'for i = 0, 15 do assert(vim.g["terminal_color_" .. i]) end',
        ]))
        # -l exits nonzero on assertion failures and never opens a UI.
        run(['nvim', '--headless', '-u', 'NONE', '-i', 'NONE', '-l', str(lua_test)], env)
        print(f'{mode}: nine targets, JSON/TOML/plist/SVG/palette checks, Vim and Neovim passed')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/chromasync')
    parser.add_argument('--output', type=Path, help='Retain generated artifacts for inspection')
    args = parser.parse_args()
    for editor in ('vim', 'nvim'):
        if not shutil.which(editor):
            parser.error(f'{editor} is required for native loading checks')
    if args.output:
        args.output.mkdir(parents=True, exist_ok=True)
        verify(args.binary.resolve(), args.output.resolve())
    else:
        with tempfile.TemporaryDirectory(prefix='chromasync-targets-') as temp:
            verify(args.binary.resolve(), Path(temp))


if __name__ == '__main__':
    main()
