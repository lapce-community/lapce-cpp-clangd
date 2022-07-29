use std::{
    error::Error,
    fs::{self, File},
    io,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use lapce_plugin::{register_plugin, send_notification, start_lsp, LapcePlugin};
mod errors;

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
    clangd_path: Option<String>,
    options: Option<Value>,
}

register_plugin!(State);

const CLANGD_VER: &str = "14.0.3";

fn check_and_download(os: String) -> Result<PathBuf, Box<dyn Error>> {
    let zip_file = match os.as_str() {
        "macos" => format!("clangd-mac-{CLANGD_VER}.zip"),
        "linux" => format!("clangd-linux-{CLANGD_VER}.zip"),
        "windows" => format!("clangd-windows-{CLANGD_VER}.zip"),
        _ => return Err(Box::new(errors::UnsupportOsError { os })),
    };

    let zip_file = PathBuf::from(zip_file);

    let server_path = PathBuf::from(format!("clangd_{CLANGD_VER}"))
        .join("bin")
        .join(match os.as_str() {
            "windows" => "clangd.exe",
            _ => "clangd",
        });

    // ! We need permission system so we can do stuff like HTTP requests to grab info about
    // ! latest releases, and download them or notify user about updates

    // let response =
    //     futures::executor::block_on(reqwest::get("https://api.github.com/repos/clangd/clangd/releases/latest")).expect("request failed");

    // let api_resp = futures::executor::block_on(response
    //     .json::<GHAPIResponse>()).expect("failed to deserialise api response");

    // let mut download_asset = Asset {
    //     ..Default::default()
    // };
    // for asset in api_resp.assets {
    //     match asset.name.strip_prefix("clangd-") {
    //         Some(asset_name) => match asset_name.starts_with(info.os.as_str()) {
    //             true => download_asset = asset,
    //             false => continue,
    //         },
    //         None => continue,
    //     }
    // }

    // if download_asset.browser_download_url.is_empty() || download_asset.name.is_empty() {
    //     panic!("failed to find clangd in release")
    // }

    // let zip_file = PathBuf::from(download_asset.name);

    let lock_file = PathBuf::from("download.lock");
    send_notification(
        "lock_file",
        &json!({
            "path": &lock_file,
        }),
    );

    if !PathBuf::from(&server_path).exists() {
        let download_url = format!(
            "https://github.com/clangd/clangd/releases/download/{CLANGD_VER}/{}",
            zip_file.display()
        );

        send_notification(
            "download_file",
            &json!({
                "url": download_url,
                "path": zip_file,
            }),
        );

        if !zip_file.exists() {
            return Err(Box::new(errors::DownloadError {}));
        }

        let mut zip = zip::ZipArchive::new(File::open(&zip_file)?)?;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }

        send_notification(
            "make_file_executable",
            &json!({
                "path": server_path.to_str().unwrap(),
            }),
        );

        _ = std::fs::remove_file(&zip_file);
    }
    _ = std::fs::remove_file(&lock_file);

    Ok(server_path)
}

// fn find_clangd_in_path() -> Option<PathBuf> {
//     // ! Need to figure out how the sandbox works to use clangd
//     // ! provided by system (package managers, etc.)
//     match env::var_os("PATH") {
//         Some(paths) => {
//             for path in env::split_paths(&paths) {
//                 if let Ok(dir) = std::path::Path::new(path.as_path()).read_dir() {
//                     for file in dir.flatten() {
//                         if let Ok(server) = file.file_name().into_string() {
//                             if server == "clangd" {
//                                 return Some(file.path());
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//         None => {}
//     }
//     None
// }

fn find_clangd_by_configuration(
    clangd_path: Option<String>,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    match clangd_path {
        Some(path) => {
            if path.len() == 0 {
                Ok(None)
            } else {
                let server_path = PathBuf::from(path);
                // if !server_path.exists() {
                    // Err(Box::new(errors::FileNotFound { path: server_path }))
                // } else {
                    Ok(Some(server_path))
                // }
            }
        }
        None => Ok(None),
    }
}

impl LapcePlugin for State {
    fn initialize(&mut self, info: serde_json::Value) {
        eprintln!("Starting lapce-cpp");
        let info = serde_json::from_value::<PluginInfo>(info).unwrap();

        // Find clangd in the following order
        // 1. User configuration.
        // 2. In the `PATH` enviroment variable (seems impossible becauseof sandbox).
        // 3. Download by ourselves.
        let server_path = find_clangd_by_configuration(info.configuration.clangd_path)
            .unwrap()
            // .or_else(|| find_clangd_in_path())
            .or_else(|| Some(check_and_download(info.os).unwrap()))
            .unwrap();

        eprintln!("LSP server path: {}", server_path.display());

        start_lsp(
            server_path.to_str().unwrap(),
            "c",
            info.configuration.options.clone(),
        );
        start_lsp(
            server_path.to_str().unwrap(),
            "cpp",
            info.configuration.options,
        )
    }
}

// // GitHub API response
// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct GHAPIResponse {
//     pub url: String,
//     pub assets_url: String,
//     pub upload_url: String,
//     pub html_url: String,
//     pub id: i64,
//     pub author: Option<Value>,
//     pub node_id: String,
//     pub tag_name: String,
//     pub target_commitish: String,
//     pub name: String,
//     pub draft: bool,
//     pub prerelease: bool,
//     pub created_at: Option<Value>,
//     pub published_at: Option<Value>,
//     pub assets: Vec<Asset>,
//     pub tarball_url: String,
//     pub zipball_url: String,
//     pub body: String,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// pub struct Asset {
//     pub url: String,
//     pub id: i64,
//     pub node_id: String,
//     pub name: String,
//     pub label: String,
//     pub uploader: Option<Value>,
//     pub content_type: String,
//     pub state: String,
//     pub size: i64,
//     pub download_count: i64,
//     pub created_at: String,
//     pub updated_at: String,
//     pub browser_download_url: String,
// }
