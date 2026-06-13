import os
import shutil
import zipfile
import requests

def downloadFiles(link_discord, link_telegram, ui, base_dir):
    os.makedirs(base_dir, exist_ok=True)

    zip_path = os.path.join(base_dir, "zapret.zip")
    extract_path = os.path.join(base_dir, "zapret_extracted")

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
                print("Found asset:", asset["name"])
                if asset["name"].lower().endswith(".zip"):
                    zip_asset = asset
                    break

            if zip_asset is None:
                raise Exception("ZIP файл не найден")

            zip_url = zip_asset["browser_download_url"]
            print("ZIP URL:", zip_url)

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

            print("ZIP downloaded")
            ui.update_status("Распаковка zapret...")

            if os.path.exists(extract_path):
                shutil.rmtree(extract_path)

            with zipfile.ZipFile(zip_path, "r") as archive:
                archive.extractall(extract_path)

            os.remove(zip_path)
            print("ZIP extracted")

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
            print("TG asset:", asset["name"])
            if asset["name"].lower().endswith(".exe"):
                exe_asset = asset
                break

        if exe_asset is None:
            raise Exception("В релизе TG-WS-PROXY отсутствует .exe файл")

        tg_name = exe_asset["name"]
        tg_url = exe_asset["browser_download_url"]
        tg_path = os.path.join(base_dir, tg_name)

        print("Selected EXE:", tg_name)
        print("EXE URL:", tg_url)

        if os.path.exists(tg_path):
            print("EXE готов")
        else:
            ui.update_status("Скачивание TG-WS-PROXY...")

            response = requests.get(
                tg_url,
                headers=headers,
                stream=True,
                timeout=(10, 120)
            )
            response.raise_for_status()

            print("Content-Length:", response.headers.get("Content-Length"))

            with open(tg_path, "wb") as file:
                for chunk in response.iter_content(8192):
                    if chunk:
                        file.write(chunk)

            print("EXE установлен")

        ui.update_status("Установка завершена!", is_error=False)
        print("Успешная установка")

    except Exception as e:
        print("Ошибка:", repr(e))
        ui.update_status(f"Ошибка: {e}", is_error=True)