import sys
import ctypes
from PySide6.QtWidgets import QApplication
from ui.main_window import FreeNet
from core.services import kill_services

def main():
    try:
        myappid = "FreeNetOrg.FreeNetApp.Version.0.2.0"
        ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)
    except:
        pass

    app = QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)
    window = FreeNet()
    window.show()
    try:
        sys.exit(app.exec())
    finally:
        kill_services()

if __name__ == "__main__":
    main()