import sys
import ctypes
import traceback
from PySide6.QtWidgets import QApplication, QMessageBox

def main():
    app = QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)

    try:
        myappid = "FreeNetOrg.FreeNetApp.Version.0.2.0"
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)
    except:
        pass

    try:
        from ui.main_window import FreeNet

        window = FreeNet()
        window.show()
        def cleanup():
            try:
                from core.services import kill_services
                kill_services()
            except:
                pass

        app.aboutToQuit.connect(cleanup)

    except Exception as e:
        error_msg = traceback.format_exc()
        error_box = QMessageBox()
        error_box.setIcon(QMessageBox.Critical)
        error_box.setWindowTitle("Критическая ошибка запуска")
        error_box.setText(f"Приложение не смогло инициализироваться:\n\n{e}")
        error_box.setDetailedText(error_msg)
        error_box.exec()
        sys.exit(1)

    sys.exit(app.exec())

if __name__ == "__main__":
    main()