import os
import subprocess
import urllib.request
from PySide6.QtCore import QSettings
from config import ZAPRET_LINK, TG_LINK

HOSTS_URL = "https://raw.githubusercontent.com/Internet-Helper/GeoHideDNS/refs/heads/main/hosts/hosts"
WINDOWS_HOSTS_PATH = r"C:\Windows\System32\drivers\etc\hosts"
MARKER = "# === GeoHideDNS Start ==="

def update_hosts_file() -> None:
    try:
        with urllib.request.urlopen(HOSTS_URL, timeout=10) as response:
            new_hosts = response.read().decode('utf-8')
            
        existing_content = ""
        if os.path.exists(WINDOWS_HOSTS_PATH):
            with open(WINDOWS_HOSTS_PATH, "r", encoding="utf-8", errors="ignore") as f:
                existing_content = f.read()

        if MARKER in existing_content:
            clean_content = existing_content.split(MARKER)[0].strip()
        else:
            clean_content = existing_content.strip()

        with open(WINDOWS_HOSTS_PATH, "w", encoding="utf-8") as f:
            f.write(clean_content)
            f.write("\n\n" + MARKER + "\n")
            f.write(new_hosts)
                
            f.write("\n# === GeoHideDNS End ===\n")
        os.system("ipconfig /flushdns")
    except:
        pass

def kill_services() -> None:
    CREATE_NO_WINDOW = 0x08000000
    subprocess.run("taskkill /f /im winws.exe", shell=True, creationflags=CREATE_NO_WINDOW, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run("taskkill /f /im TgWsProxy_windows.exe", shell=True, creationflags=CREATE_NO_WINDOW, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

def run_services(base_dir: str) -> None:
    CREATE_NO_WINDOW = 0x08000000
    kill_services()
    
    settings = QSettings("FreeNetOrg", "FreeNet")
    bat_name = settings.value("selected_bat", "general (ALT12).bat")
    bat_dir = os.path.join(base_dir, "zapret_extracted")
    bat_path = os.path.join(bat_dir, bat_name)
    
    if os.path.exists(bat_path):
        subprocess.Popen(
            ["cmd.exe", "/c", bat_path],
            shell=False,
            cwd=bat_dir,
            creationflags=CREATE_NO_WINDOW
        )
        
    exe_path = os.path.join(base_dir, "TgWsProxy_windows.exe")
    if os.path.exists(exe_path):
        subprocess.Popen(
            [exe_path],
            cwd=base_dir,
            creationflags=CREATE_NO_WINDOW,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

def start_services_logic(base_dir: str) -> tuple[bool, str]:
    try:
        run_services(base_dir)
        return True, "Сервисы запущены"
    except Exception as e:
        return False, f"Ошибка запуска: {e}"

def install_and_start_logic(ui_context) -> tuple[bool, str]:
    settings = QSettings("FreeNetOrg", "FreeNet")
    base_dir = settings.value("install_path", r"C:\zapret_downloads")
    
    try:
        update_hosts_file()
        from utils.installation import downloadFiles
        downloadFiles(ZAPRET_LINK, TG_LINK, ui_context, base_dir)
        return start_services_logic(base_dir)
    except Exception as e:
        return False, f"Ошибка при установке: {e}"