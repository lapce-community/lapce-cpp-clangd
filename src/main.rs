use std::{
  fs::{self, File},
  io::{self, Read, Write},
  path::PathBuf,
};

use anyhow::{anyhow, Result};
use lapce_plugin::{
  psp_types::{
    lsp_types::{
      request::Initialize, DocumentFilter, DocumentSelector, InitializeParams, MessageType, Url,
    },
    Request,
  },
  register_plugin, Http, LapcePlugin, VoltEnvironment, PLUGIN_RPC,
};
use serde_json::Value;
use zip::ZipArchive;

#[derive(Default)]
struct State {}

register_plugin!(State);

macro_rules! string {
  ( $x:expr ) => {
    String::from($x)
  };
}

macro_rules! ok {
  ( $x:expr ) => {
    match ($x) {
      | Ok(v) => v,
      | Err(e) => return Err(anyhow!(e)),
    }
  };
}

const CLANGD_VERSION: &str = "15.0.1";
const CPP_PATTERN: &str = "**/*.{H,hh,hpp,h++,C,cc,cpp,c++}";
const C_PATTERN: &str = "**/*.{h,c}";

fn initialize(params: InitializeParams) -> Result<()> {
  let mut cpp_pattern = string!(CPP_PATTERN);
  let mut c_pattern = string!(C_PATTERN);

  if let Some(options) = params.initialization_options.as_ref() {
    if let Some(volt) = options.get("volt") {
      if let Some(cpp_pat) = volt.get("cppPattern") {
        if let Some(cpp_pat) = cpp_pat.as_str() {
          let cpp_pat = cpp_pat.trim();
          if !cpp_pat.is_empty() {
            cpp_pattern = string!("**/*.{");
            cpp_pattern.push_str(cpp_pat);
            cpp_pattern.push('}');
          }
        }
      }
      if let Some(c_pat) = volt.get("cPattern") {
        if let Some(c_pat) = c_pat.as_str() {
          let c_pat = c_pat.trim();
          if !c_pattern.is_empty() {
            c_pattern = string!("**/*.{");
            c_pattern.push_str(c_pat);
            c_pattern.push('}');
          }
        }
      }
    }
  }

  let document_selector: DocumentSelector = vec![
    DocumentFilter {
      language: Some(string!("cpp")),
      pattern: Some(string!(cpp_pattern)),
      scheme: None,
    },
    DocumentFilter {
      language: Some(string!("c")),
      pattern: Some(string!(c_pattern)),
      scheme: None,
    },
  ];

  let mut clangd_version = string!(CLANGD_VERSION);
  let mut server_args = vec![];

  if let Some(options) = params.initialization_options.as_ref() {
    if let Some(volt) = options.get("volt") {
      if let Some(args) = volt.get("serverArgs") {
        if let Some(args) = args.as_array() {
          for arg in args {
            if let Some(arg) = arg.as_str() {
              server_args.push(string!(arg));
            }
          }
        }
      }
      if let Some(server_path) = volt.get("serverPath") {
        if let Some(server_path) = server_path.as_str() {
          if !server_path.is_empty() {
            let server_uri = ok!(Url::parse(&format!("urn:{}", server_path)));
            PLUGIN_RPC.start_lsp(
              server_uri,
              server_args,
              document_selector,
              params.initialization_options,
            );
            return Ok(());
          }
        }
      }
      if let Some(clangd_ver) = options.get("clangdVersion") {
        if let Some(clangd_ver) = clangd_ver.as_str() {
          let clangd_ver = clangd_ver.trim();
          if !clangd_ver.is_empty() {
            clangd_version = string!(clangd_ver)
          }
        }
      }
    }
  }

  PLUGIN_RPC.stderr(&format!("clangd: {clangd_version}"));

  let _ = match VoltEnvironment::architecture().as_deref() {
    | Ok("x86_64") => "x86_64",
    | Ok(v) => return Err(anyhow!("Unsupported ARCH: {}", v)),
    | Err(e) => return Err(anyhow!("Error ARCH: {}", e)),
  };

  let mut last_ver = ok!(fs::OpenOptions::new()
    .create(true)
    .write(true)
    .read(true)
    .open(".clangd_ver"));
  let mut buf = String::new();
  ok!(last_ver.read_to_string(&mut buf));

  let mut server_path = PathBuf::from(format!("clangd_{clangd_version}"));
  server_path = server_path.join("bin");

  // if buf.trim().is_empty() || buf.trim() != clangd_version {
  //   if buf.trim() != clangd_version {
  //   ok!(fs::remove_dir_all(&server_path));
  // }

  let zip_file = match VoltEnvironment::operating_system().as_deref() {
    | Ok("macos") => PathBuf::from(format!("clangd-mac-{clangd_version}.zip")),
    | Ok("linux") => PathBuf::from(format!("clangd-linux-{clangd_version}.zip")),
    | Ok("windows") => PathBuf::from(format!("clangd-windows-{clangd_version}.zip")),
    | Ok(v) => return Err(anyhow!("Unsupported OS: {}", v)),
    | Err(e) => return Err(anyhow!("Error OS: {}", e)),
  };

  let download_url = format!(
    "https://github.com/clangd/clangd/releases/download/{clangd_version}/{}",
    zip_file.display()
  );

  let mut resp = ok!(Http::get(&download_url));
  PLUGIN_RPC.stderr(&format!("STATUS_CODE: {:?}", resp.status_code));
  let body = ok!(resp.body_read_all());
  ok!(fs::write(&zip_file, body));

  let mut zip = ok!(ZipArchive::new(ok!(File::open(&zip_file))));

  for i in 0..zip.len() {
    let mut file = ok!(zip.by_index(i));
    let outpath = match file.enclosed_name() {
      | Some(path) => path.to_owned(),
      | None => continue,
    };

    if (*file.name()).ends_with('/') {
      ok!(fs::create_dir_all(&outpath));
    } else {
      if let Some(p) = outpath.parent() {
        if !p.exists() {
          ok!(fs::create_dir_all(&p));
        }
      }
      let mut outfile = ok!(File::create(&outpath));
      ok!(io::copy(&mut file, &mut outfile));
    }

    ok!(fs::remove_file(&zip_file));
  }
  // }

  ok!(last_ver.write_all(clangd_version.as_bytes()));

  match VoltEnvironment::operating_system().as_deref() {
    | Ok("windows") => {
      server_path = server_path.join("clangd.exe");
    }
    | _ => {
      server_path = server_path.join("clangd");
    }
  };

  let volt_uri = ok!(VoltEnvironment::uri());
  let server_path = match server_path.to_str() {
    | Some(v) => v,
    | None => return Err(anyhow!("server_path.to_str() failed")),
  };
  let server_uri = ok!(ok!(Url::parse(&volt_uri)).join(server_path));

  PLUGIN_RPC.start_lsp(
    server_uri,
    server_args,
    document_selector,
    params.initialization_options,
  );

  Ok(())
}

impl LapcePlugin for State {
  fn handle_request(&mut self, _id: u64, method: String, params: Value) {
    #[allow(clippy::single_match)]
    match method.as_str() {
      | Initialize::METHOD => {
        let params: InitializeParams = serde_json::from_value(params).unwrap();
        if let Err(e) = initialize(params) {
          PLUGIN_RPC.window_log_message(MessageType::ERROR, e.to_string());
          PLUGIN_RPC.window_show_message(MessageType::ERROR, e.to_string());
        };
      }
      | _ => {}
    }
  }
}
