use std::{
    fs::{self, remove_file, File},
    io,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use lapce_plugin::{register_plugin, send_notification, start_lsp, LapcePlugin};
use zip::ZipArchive;

#[derive(Default)]
struct State {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    arch: String,
    os: String,
    configuration: Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    language_id: String,
    system_lsp: bool,
    disabled: bool,
    release: Option<String>,
    options: Option<Value>,
}

register_plugin!(State);

const CLANGD_VER: &str = "14.0.3";

impl LapcePlugin for State {
    fn initialize(&mut self, info: serde_json::Value) {
        eprintln!("Starting plugin");

        let info = serde_json::from_value::<PluginInfo>(info).unwrap();

        if info.configuration.disabled {
            eprintln!("plugin disabled, returning");
        }

        let exec_path = if info.configuration.system_lsp {
            eprintln!("Using system LSP");
            "clangd".to_string()
        } else {
            let clangd_version = if let Some(ver) = info.configuration.release {
                ver
            } else {
                CLANGD_VER.to_string()
            };

            eprintln!("Using clangd version: {clangd_version}");

            let _ = match info.arch.as_str() {
                "x86_64" => "x86_64",
                _ => return,
            };

            let zip_file = match info.os.as_str() {
                "macos" => format!("clangd-mac-{clangd_version}.zip"),
                "linux" => format!("clangd-linux-{clangd_version}.zip"),
                "windows" => format!("clangd-windows-{clangd_version}.zip"),
                _ => return,
            };

            let zip_file = PathBuf::from(zip_file);

            let download_url = format!(
                "https://github.com/clangd/clangd/releases/download/{clangd_version}/{}",
                zip_file.display()
            );

            let mut server_path = PathBuf::from(format!("clangd_{clangd_version}"));
            server_path = server_path.join("bin");

            match info.os.as_str() {
                "windows" => {
                    server_path = server_path.join("clangd.exe");
                }
                _ => {
                    server_path = server_path.join("clangd");
                }
            }

            let exec_path = format!("{}", server_path.display());

            let lock_file = PathBuf::from("download.lock");
            send_notification(
                "lock_file",
                &json!({
                    "path": &lock_file,
                }),
            );

            if !PathBuf::from(&server_path).exists() {
                send_notification(
                    "download_file",
                    &json!({
                        "url": download_url,
                        "path": zip_file,
                    }),
                );

                if !zip_file.exists() {
                    eprintln!("clangd download failed");
                    return;
                }

                let mut zip =
                    ZipArchive::new(File::open(&zip_file).unwrap()).expect("failed to open zip");

                for i in 0..zip.len() {
                    let mut file = zip.by_index(i).unwrap();
                    let outpath = match file.enclosed_name() {
                        Some(path) => path.to_owned(),
                        None => continue,
                    };

                    if (*file.name()).ends_with('/') {
                        fs::create_dir_all(&outpath).unwrap();
                    } else {
                        if let Some(p) = outpath.parent() {
                            if !p.exists() {
                                fs::create_dir_all(&p).unwrap();
                            }
                        }
                        let mut outfile = File::create(&outpath).unwrap();
                        io::copy(&mut file, &mut outfile).unwrap();
                    }
                }

                send_notification(
                    "make_file_executable",
                    &json!({
                        "path": exec_path,
                    }),
                );

                _ = remove_file(&zip_file);
            }
            _ = remove_file(&lock_file);

            exec_path
        };

        eprintln!("LSP server path: {}", exec_path);

        start_lsp(
            &exec_path,
            "c",
            info.configuration.options.clone(),
            info.configuration.system_lsp,
        );
        start_lsp(
            &exec_path,
            "cpp",
            info.configuration.options,
            info.configuration.system_lsp,
        )
    }
}
