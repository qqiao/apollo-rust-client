[English](../en/Configuration.md) | [中文简体](../zh-CN/Configuration.md)
[返回首頁](Home.md)

## 設定

客戶端通过 `ClientConfig` 結構體/類別支援以下設定選項：

- `app_id`: 您在 Apollo 中的應用 ID。
- `cluster`: 叢集名稱 (預設為 "`default`")。
- `secret`: 給定 `app_id` 的可選金鑰。
- `config_server`: 設定伺服器的位址。
- `cache_dir`: 用于儲存本地快取的目錄。
  - 對于非 WASM 目標（原生 Rust 應用程式）：如果未指定，則預設為建構為 `/opt/data/{app_id}/config-cache` 的路徑。
  - 對于 WASM 目標：此欄位是可選的，預設為 `None`；檔案系統快取在瀏覽器環境中通常不適用或不使用。
- `label`: 目前實例的標籤。用于在灰度發布中識別目前實例。
- `ip`: 您應用程式的 IP 位址。用于在灰度發布中識別目前實例。
- `allow_insecure_https`: 是否允許不安全的 HTTPS 連接（自簽名憑證）。用于公司內部網路或開發環境。
