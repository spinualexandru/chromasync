// Optional native QA: see README.md in this directory.
#include <QApplication>
#include <QCheckBox>
#include <QComboBox>
#include <QLabel>
#include <QLineEdit>
#include <QPainter>
#include <QProgressBar>
#include <QPushButton>
#include <QRadioButton>
#include <QSettings>
#include <QSlider>
#include <QStyleFactory>
#include <QStyleOptionFocusRect>
#include <QVBoxLayout>
#include <QWidget>
#include <iostream>

int main(int argc, char** argv) {
    QApplication app(argc, argv);
    if (argc != 3) {
        std::cerr << "usage: kvantum-verify CONFIG.kvconfig OUTPUT.png\n";
        return 1;
    }
    auto* style = QStyleFactory::create("kvantum");
    if (!style) {
        std::cerr << "Kvantum style plugin is required\n";
        return 1;
    }
    app.setStyle(style);
    // A real application asks the style for its standard palette.
    app.setPalette(style->standardPalette());
    QSettings settings(argv[1], QSettings::IniFormat);
    settings.beginGroup("GeneralColors");
    const auto palette = app.palette();
    for (auto [role, key] : {
        std::pair{QPalette::Window, "window.color"},
        std::pair{QPalette::WindowText, "window.text.color"},
        std::pair{QPalette::Base, "base.color"},
        std::pair{QPalette::Text, "text.color"},
        std::pair{QPalette::Highlight, "highlight.color"},
        std::pair{QPalette::HighlightedText, "highlight.text.color"},
        std::pair{QPalette::Button, "button.color"},
        std::pair{QPalette::ButtonText, "button.text.color"},
    }) {
        if (palette.color(role) != QColor(settings.value(key).toString())) {
            std::cerr << "Kvantum palette did not load " << key << '\n';
            return 1;
        }
    }
    // Focus elements have no state suffix in Kvantum's rendering API.
    QImage focusImage(120, 40, QImage::Format_ARGB32);
    focusImage.fill(Qt::transparent);
    QPainter painter(&focusImage);
    QStyleOptionFocusRect focus;
    focus.rect = QRect(4, 4, 100, 30);
    focus.state = QStyle::State_Enabled | QStyle::State_HasFocus | QStyle::State_KeyboardFocusChange;
    style->drawPrimitive(QStyle::PE_FrameFocusRect, &focus, &painter);
    painter.end();
    // Validate visibility without depending on the selected palette's exact accent.
    bool visibleFocus = false;
    for (int y = 0; y < focusImage.height(); ++y)
        for (int x = 0; x < focusImage.width(); ++x)
            visibleFocus |= focusImage.pixelColor(x, y).alpha() > 0;
    if (!visibleFocus) {
        std::cerr << "Kvantum keyboard focus frame is missing\n";
        return 1;
    }
    QWidget window;
    window.setWindowTitle("Chromasync Kvantum audit");
    auto* layout = new QVBoxLayout(&window);
    auto* title = new QLabel("Chromasync: real Kvantum widgets");
    title->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Maximum);
    layout->addWidget(title);
    auto* edit = new QLineEdit("Selected text and editable content");
    layout->addWidget(edit);
    auto* button = new QPushButton("Keyboard focus / action");
    layout->addWidget(button);
    auto* disabled = new QPushButton("Disabled action");
    disabled->setEnabled(false);
    layout->addWidget(disabled);
    auto* check = new QCheckBox("Checked option");
    check->setChecked(true);
    layout->addWidget(check);
    layout->addWidget(new QCheckBox("Unchecked option"));
    auto* radio = new QRadioButton("Selected radio option");
    radio->setChecked(true);
    layout->addWidget(radio);
    auto* combo = new QComboBox;
    combo->addItems({"First choice", "Second choice"});
    layout->addWidget(combo);
    auto* progress = new QProgressBar;
    progress->setValue(65);
    layout->addWidget(progress);
    auto* slider = new QSlider(Qt::Horizontal);
    slider->setValue(65);
    layout->addWidget(slider);
    window.resize(480, 340);
    window.show();
    button->setFocus();
    app.processEvents();
    if (!window.grab().save(argv[2])) {
        return 1;
    }
    std::cout << "Kvantum palette load and widget render passed\n";
}
