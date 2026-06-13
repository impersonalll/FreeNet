import os
from PySide6.QtCore import Qt, Signal, QSettings
from PySide6.QtWidgets import (QWidget, QVBoxLayout, QHBoxLayout, 
                             QPushButton, QLabel, QFileDialog, QComboBox)

class SettingsPage(QWidget):
    back_requested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.parent = parent
        self.settings = QSettings("FreeNetOrg", "FreeNet")
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(40, 20, 40, 20)
        layout.setAlignment(Qt.AlignmentFlag.AlignTop)

        header_layout = QHBoxLayout()
        
        back_button = QPushButton("← Назад")
        back_button.setProperty("class", "IconButton")
        back_button.setCursor(Qt.CursorShape.PointingHandCursor)
        back_button.clicked.connect(self.back_requested.emit)
        
        settings_title = QLabel("Настройки")
        settings_title.setStyleSheet("font-size: 20px; font-weight: bold; color: #ffffff;")

        header_layout.addWidget(back_button)
        header_layout.addWidget(settings_title)
        header_layout.addStretch()
        layout.addLayout(header_layout)
        layout.addSpacing(20)

        script_label = QLabel("Скрипт обхода блокировок (.bat):")
        script_label.setStyleSheet("font-size: 14px; color: #a5a1b8;")
        layout.addWidget(script_label)
        layout.addSpacing(5)

        self.script_combo = QComboBox()
        self.script_combo.setCursor(Qt.CursorShape.PointingHandCursor)
        self.script_combo.setStyleSheet("""
            QComboBox {
                background-color: #252336;
                color: #d1cedd;
                border: 1px solid #33314a;
                border-radius: 8px;
                padding: 10px;
                font-size: 14px;
            }
            QComboBox::drop-down {
                border: none;
                padding-right: 10px;
            }
            QComboBox QAbstractItemView {
                background-color: #252336;
                color: #d1cedd;
                selection-background-color: #6352eb;
            }
        """)
        
        saved_path = self.settings.value("install_path", r"C:\zapret_downloads")
        self.update_scripts_list(saved_path)
        self.script_combo.currentIndexChanged.connect(self.save_selected_script)
        layout.addWidget(self.script_combo)
        layout.addSpacing(15)

        path_label = QLabel("Место установки компонентов:")
        path_label.setStyleSheet("font-size: 14px; color: #a5a1b8;")
        layout.addWidget(path_label)
        layout.addSpacing(5)

        path_layout = QHBoxLayout()
        
        self.path_edit = QLabel(saved_path)
        self.path_edit.setStyleSheet("""
            QLabel {
                background-color: #252336;
                color: #d1cedd;
                border: 1px solid #33314a;
                border-radius: 8px;
                padding: 10px;
                font-size: 14px;
            }
        """)
        
        browse_btn = QPushButton("Обзор...")
        browse_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        browse_btn.setStyleSheet("""
            QPushButton {
                background-color: #6352eb;
                color: white;
                border: none;
                border-radius: 8px;
                padding: 10px 20px;
                font-size: 14px;
                font-weight: bold;
            }
            QPushButton:hover {
                background-color: #7b66ff;
            }
        """)
        browse_btn.clicked.connect(self.choose_directory)

        path_layout.addWidget(self.path_edit, stretch=1)
        path_layout.addWidget(browse_btn)
        layout.addLayout(path_layout)

    def toggle_autostart(self, checked):
        if checked:
            self.auto_start_btn.setText("Автозапуск вместе с Windows: ВКЛ")
        else:
            self.auto_start_btn.setText("Автозапуск вместе с Windows: ВЫКЛ")

    def update_scripts_list(self, base_path):
        self.script_combo.clear()
        zapret_folder = os.path.join(base_path, "zapret_extracted")
        
        if os.path.exists(zapret_folder):
            files = [f for f in os.listdir(zapret_folder) if f.lower().endswith('.bat')]
            if files:
                self.script_combo.addItems(files)
                saved_script = self.settings.value("selected_bat", "")
                if saved_script in files:
                    self.script_combo.setCurrentText(saved_script)
                return
                
        self.script_combo.addItem("general (ALT12).bat")

    def save_selected_script(self):
        current_text = self.script_combo.currentText()
        if current_text:
            self.settings.setValue("selected_bat", current_text)

    def choose_directory(self):
        dir_path = QFileDialog.getExistingDirectory(
            self, 
            "Выбрать папку установки", 
            self.path_edit.text()
        )
        if dir_path:
            normalized_path = os.path.normpath(dir_path)
            self.path_edit.setText(normalized_path)
            self.settings.setValue("install_path", normalized_path)
            self.update_scripts_list(normalized_path)