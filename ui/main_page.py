from PySide6.QtCore import Qt, Signal
from PySide6.QtWidgets import QWidget, QVBoxLayout, QPushButton, QLabel

class MainPage(QWidget):
    start_requested = Signal()
    stop_requested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.parent = parent
        self.is_active = False
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(40, 40, 40, 40)
        layout.setAlignment(Qt.AlignmentFlag.AlignCenter)

        self.power_button = QPushButton("ON")
        self.power_button.setObjectName("PowerButton")
        self.power_button.setCheckable(True)
        self.power_button.setFixedSize(150, 150)
        self.power_button.setCursor(Qt.CursorShape.PointingHandCursor)
        self.power_button.clicked.connect(self.handle_action)
        layout.addWidget(self.power_button, alignment=Qt.AlignmentFlag.AlignCenter)
        layout.addSpacing(30)

        self.status_label = QLabel("Сервисы отключены")
        self.status_label.setObjectName("StatusLabel")
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(self.status_label)

    def handle_action(self):
        if not self.is_active:
            self.power_button.setEnabled(False)
            self.update_status("Проверка компонентов и запуск...")
            self.start_requested.emit()
        else:
            self.power_button.setEnabled(False)
            self.update_status("Остановка служб...")
            self.stop_requested.emit()

    def update_status(self, text, is_error=False):
        self.status_label.setText(text)
        if is_error:
            self.status_label.setStyleSheet("font-size: 14px; color: #e53935;")
            self.power_button.setEnabled(True)
            self.power_button.setChecked(False)
        else:
            self.status_label.setStyleSheet("font-size: 14px; color: #747185;")

    def set_connected_state(self):
        self.is_active = True
        self.power_button.setEnabled(True)
        self.power_button.setChecked(True)
        self.power_button.setText("OFF")
        self.update_status("Сервисы подключены. Все службы работают.")

    def disconnect_services(self):
        self.is_active = False
        self.power_button.setEnabled(True)
        self.power_button.setChecked(False)
        self.power_button.setText("ON")
        self.update_status("Сервисы отключены")