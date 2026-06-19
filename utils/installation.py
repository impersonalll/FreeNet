import os
import shutil
import zipfile
import requests

def downloadFiles(link_discord, link_telegram, ui, base_dir):
    os.makedirs(base_dir, exist_ok=True)

    zip_path = os.path.join(base_dir, "zapret.zip")
    extract_path = os.path.join(base_dir, "zapret_extracted")
    
    tg_path = os.path.join(base_dir, "TgWsProxy_windows.exe")

    headers = {
        "User-Agent": "FreeNet/1.0"
    }

    try:
        print("=" * 50)
        print("START INSTALLATION")
        print("=" * 50)

        if os.path.exists(extract_path) and os.listdir(extract_path):
            print("Zapret already installed")
        else:
            ui.update_status("Получение релиза zapret...")

            release_response = requests.get(
                link_discord,
                headers=headers,
                timeout=15
            )
            release_response.raise_for_status()
            release = release_response.json()

            zip_asset = None
            for asset in release.get("assets", []):
                if asset["name"].lower().endswith(".zip"):
                    zip_asset = asset
                    break

            if zip_asset is None:
                raise Exception("ZIP файл не найден в релизе Zapret")

            zip_url = zip_asset["browser_download_url"]
            ui.update_status("Скачивание zapret...")

            response = requests.get(
                zip_url,
                headers=headers,
                stream=True,
                timeout=(10, 60)
            )
            response.raise_for_status()

            with open(zip_path, "wb") as file:
                for chunk in response.iter_content(8192):
                    if chunk:
                        file.write(chunk)

            ui.update_status("Распаковка zapret...")

            if os.path.exists(extract_path):
                shutil.rmtree(extract_path)
            os.makedirs(extract_path, exist_ok=True)

            temp_extract_path = os.path.join(base_dir, "zapret_temp")
            if os.path.exists(temp_extract_path):
                shutil.rmtree(temp_extract_path)

            with zipfile.ZipFile(zip_path, "r") as archive:
                archive.extractall(temp_extract_path)

            internal_items = os.listdir(temp_extract_path)
            
            if len(internal_items) == 1 and os.path.isdir(os.path.join(temp_extract_path, internal_items[0])):
                inner_folder_path = os.path.join(temp_extract_path, internal_items[0])
                for item in os.listdir(inner_folder_path):
                    shutil.move(os.path.join(inner_folder_path, item), extract_path)
            else:
                for item in internal_items:
                    shutil.move(os.path.join(temp_extract_path, item), extract_path)

            shutil.rmtree(temp_extract_path)
            if os.path.exists(zip_path):
                os.remove(zip_path)

        if os.path.exists(tg_path):
            print("EXE готов")
        else:
            ui.update_status("Получение релиза TG...")


            tg_release_response = requests.get(
                link_telegram,
                headers=headers,
                timeout=15
            )
            tg_release_response.raise_for_status()
            tg_release = tg_release_response.json()

            exe_asset = None
            for asset in tg_release.get("assets", []):
                if asset["name"].lower().endswith(".exe"):
                    exe_asset = asset
                    break

            if exe_asset is None:
                raise Exception("В релизе TG-WS-PROXY отсутствует .exe файл")

            tg_url = exe_asset["browser_download_url"]
            ui.update_status("Скачивание TG-WS-PROXY...")

            response = requests.get(tg_url, headers=headers, stream=True, timeout=(10, 120))
            response.raise_for_status()

            with open(tg_path, "wb") as file:
                for chunk in response.iter_content(8192):
                    if chunk:
                        file.write(chunk)

        ui.update_status("Установка завершена!", is_error=False)

    except Exception as e:
        ui.update_status(f"Ошибка: {e}", is_error=True)
        raise e