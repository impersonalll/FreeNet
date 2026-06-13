import threading
import ctypes
import os
import sys
import psutil
from PySide6.QtCore import Qt, QPoint, QTimer
from PySide6.QtGui import QIcon, QAction
from PySide6.QtWidgets import (QWidget, QVBoxLayout, QHBoxLayout, 
                             QPushButton, QLabel, QSystemTrayIcon, 
                             QMenu, QStackedWidget, QApplication)

from ui.main_page import MainPage
from ui.settings_page import SettingsPage
from core.services import install_and_start_logic, kill_services


def get_resource_path(relative_path):
    if hasattr(sys, '_MEIPASS'):
        return os.path.join(sys._MEIPASS, relative_path)
    return os.path.join(os.path.abspath("."), relative_path)

class FreeNet(QWidget):
    def __init__(self):
        super().__init__()
        self.old_pos = None
        self.init_ui()
        
        self.sys_timer = QTimer(self)
        self.sys_timer.timeout.connect(self.update_system_stats)
        self.sys_timer.start(1000)

    def create_tray(self):
        self.tray_icon = QSystemTrayIcon(self)
        self.tray_icon.setIcon(QIcon(get_resource_path("assets/icon.ico")))

        tray_menu = QMenu()
        show_action = QAction("Открыть", self)
        quit_action = QAction("Выход", self)

        show_action.triggered.connect(self.show_window)
        quit_action.triggered.connect(self.safe_quit)

        tray_menu.addAction(show_action)
        tray_menu.addSeparator()
        tray_menu.addAction(quit_action)

        self.tray_icon.setContextMenu(tray_menu)
        self.tray_icon.activated.connect(self.tray_activated)
        self.tray_icon.show()

    def show_window(self):
        self.showNormal()
        self.activateWindow()
        self.raise_()

    def tray_activated(self, reason):
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.show_window()

    def hide_to_tray(self):
        self.hide()

    def closeEvent(self, event):
        event.ignore()
        self.hide()

    def safe_quit(self):
        self.tray_icon.hide()
        kill_services()
        QApplication.quit()

    def init_ui(self):
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.resize(850, 650)
        self.create_tray()
        
        self.setWindowIcon(QIcon(get_resource_path("assets/icon.ico")))
        
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(0, 0, 0, 0)

        window_frame = QWidget()
        window_frame.setObjectName("WindowFrame")

        window_frame.setStyleSheet("""
            QWidget#WindowFrame { background-color: #1a1926; border-radius: 16px; border: 1px solid #2d2b3d; }
            QLabel { color: #ffffff; font-family: 'Segoe UI', sans-serif; }
            #TitleLabel { font-size: 16px; font-weight: bold; color: #a5a1b8; }
            .IconButton { background-color: transparent; color: #a5a1b8; border: none; font-size: 16px; font-weight: bold; padding: 5px 10px; }
            .IconButton:hover { color: #ffffff; background-color: #2d2b3d; border-radius: 8px; }
            #CloseButton:hover { background-color: #e53935; color: white; }
            QComboBox { background-color: #252336; border: 1px solid #33314a; border-radius: 8px; padding: 8px 12px; color: #d1cedd; font-family: 'Segoe UI', sans-serif; min-width: 140px; }
            QComboBox:hover { border-color: #6352eb; }
            QComboBox::drop-down { subcontrol-origin: padding; subcontrol-position: top right; width: 25px; border-left-width: 0px; }
            QComboBox QAbstractItemView { background-color: #252336; border: 1px solid #33314a; color: #d1cedd; selection-background-color: #6352eb; selection-color: white; outline: 0px; }
            #PowerButton { background: qlineargradient(x1:0, y1:0, x2:1, y2:1, stop:0 #6352eb, stop:1 #4333c7); border: 4px solid #2d2b3d; border-radius: 75px; color: white; font-size: 36px; font-weight: bold; }
            #PowerButton:checked { background: qlineargradient(x1:0, y1:0, x2:1, y2:1, stop:0 #00e676, stop:1 #00b0ff); border: 4px solid #2d2b3d; }
            #PowerButton:hover { border-color: #7b66ff; }
            #StatusLabel { font-size: 14px; color: #747185; }
            #BottomBar { background-color: #151421; border-top: 1px solid #2d2b3d; border-bottom-left-radius: 16px; border-bottom-right-radius: 16px; }
            #StatLabel { font-size: 12px; color: #8a879a; font-weight: 500; margin-right: 20px; }
        """)

        frame_layout = QVBoxLayout(window_frame)
        frame_layout.setContentsMargins(0, 0, 0, 0)

        self.title_bar = QWidget()
        title_bar_layout = QHBoxLayout(self.title_bar)
        title_bar_layout.setContentsMargins(20, 15, 15, 10)

        title_label = QLabel("FreeNet")
        title_label.setObjectName("TitleLabel")

        settings_button = QPushButton("...")
        settings_button.setProperty("class", "IconButton")
        settings_button.setCursor(Qt.CursorShape.PointingHandCursor)
        settings_button.clicked.connect(lambda: self.stacked_widget.setCurrentIndex(1))

        close_button = QPushButton("×")
        close_button.setObjectName("CloseButton")
        close_button.setProperty("class", "IconButton")
        close_button.setCursor(Qt.CursorShape.PointingHandCursor)
        close_button.clicked.connect(self.hide_to_tray)

        title_bar_layout.addWidget(title_label)
        title_bar_layout.addStretch()
        title_bar_layout.addWidget(settings_button)
        title_bar_layout.addWidget(close_button)
        
        frame_layout.addWidget(self.title_bar)

        self.stacked_widget = QStackedWidget()

        self.main_page = MainPage(self)
        self.settings_page = SettingsPage(self)

        self.main_page.start_requested.connect(self.handle_power_service)
        self.main_page.stop_requested.connect(self.handle_stop_service)
        self.settings_page.back_requested.connect(lambda: self.stacked_widget.setCurrentIndex(0))

        self.stacked_widget.addWidget(self.main_page)
        self.stacked_widget.addWidget(self.settings_page)

        frame_layout.addWidget(self.stacked_widget, stretch=1)

        bottom_bar = QWidget()
        bottom_bar.setObjectName("BottomBar")
        bottom_bar_layout = QHBoxLayout(bottom_bar)
        bottom_bar_layout.setContentsMargins(20, 10, 20, 10)

        self.uac_label = QLabel("Доступ: Проверка...")
        self.uac_label.setObjectName("StatLabel")
        
        self.process_label = QLabel("Службы: Остановлены")
        self.process_label.setObjectName("StatLabel")

        bottom_bar_layout.addWidget(self.uac_label)
        bottom_bar_layout.addWidget(self.process_label)
        bottom_bar_layout.addStretch()

        frame_layout.addWidget(bottom_bar)
        main_layout.addWidget(window_frame)

    def handle_power_service(self):
        threading.Thread(target=self._bg_install_and_start, daemon=True).start()

    def handle_stop_service(self):
        threading.Thread(target=self._bg_stop_services, daemon=True).start()

    def _bg_install_and_start(self):
        success, message = install_and_start_logic(self.main_page)
        if success:
            self.main_page.set_connected_state()
        else:
            self.main_page.update_status(message, is_error=True)

    def _bg_stop_services(self):
        kill_services()
        self.main_page.disconnect_services()

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            if self.title_bar.geometry().contains(event.pos()):
                self.old_pos = event.globalPosition().toPoint()
            else:
                self.old_pos = None

    def mouseMoveEvent(self, event):
        if self.old_pos is not None:
            delta = QPoint(event.globalPosition().toPoint() - self.old_pos)
            self.move(self.x() + delta.x(), self.y() + delta.y())
            self.old_pos = event.globalPosition().toPoint()

    def mouseReleaseEvent(self, event):
        self.old_pos = None
        
    def update_system_stats(self):
        try:
            is_admin = ctypes.windll.shell32.IsUserAnAdmin() != 0
            uac_text = "Доступ: Администратор" if is_admin else "Доступ: Пользователь (Ограничен)"
            self.uac_label.setStyleSheet("color: #00e676;" if is_admin else "color: #ff3b30;")
        except:
            uac_text = "Доступ: Неизвестно"

        winws_active = False
        proxy_active = False
        
        try:
            for proc in psutil.process_iter(['name']):
                name = proc.info['name'].lower() if proc.info['name'] else ""
                if "winws.exe" in name:
                    winws_active = True
                if "tgwsproxy" in name:
                    proxy_active = True
        except:
            pass

        if winws_active and proxy_active:
            proc_text = "Службы: Активны (Zapret + Proxy)"
            self.process_label.setStyleSheet("color: #00b0ff;")
            if not self.main_page.is_active:
                self.main_page.set_connected_state()
        elif winws_active or proxy_active:
            proc_text = "Службы: Частичный запуск"
            self.process_label.setStyleSheet("color: #ff9100;")
        else:
            proc_text = "Службы: Остановлены"
            self.process_label.setStyleSheet("color: #8a879a;")
            if self.main_page.is_active:
                self.main_page.disconnect_services()

        self.uac_label.setText(uac_text)
        self.process_label.setText(proc_text)
