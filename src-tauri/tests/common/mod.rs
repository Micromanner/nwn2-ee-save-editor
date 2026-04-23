use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

use app_lib::config::NWN2Paths;
use app_lib::loaders::GameDataLoader;
use app_lib::services::ResourceManager;

use app_lib::services::item_property_decoder::ItemPropertyDecoder;

#[allow(dead_code)]
pub struct TestContext {
    pub loader: GameDataLoader,
    pub resource_manager: Arc<RwLock<ResourceManager>>,
    pub decoder: ItemPropertyDecoder,
    pub _temp_dir: TempDir,
}

#[allow(dead_code)]
pub fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[allow(dead_code)]
pub fn load_test_gff(name: &str) -> Vec<u8> {
    let path = if name.ends_with(".bic") {
        fixtures_path().join("gff").join(name)
    } else {
        fixtures_path().join("saves").join(name)
    };

    std::fs::read(&path).unwrap_or_else(|_| {
        panic!(
            "Failed to load fixture: {name} at {path}",
            path = path.display()
        )
    })
}

#[allow(dead_code)]
pub async fn create_test_context() -> TestContext {
    let temp = TempDir::new().unwrap();

    // Initialize Paths
    let paths = Arc::new(RwLock::new(NWN2Paths::new()));

    // Initialize Resource Manager
    // We need strict ownership here to initialize
    let resource_manager = Arc::new(RwLock::new(ResourceManager::new(paths.clone())));

    // Initialize ResourceManager internal state (indexing)
    {
        let mut rm = resource_manager.write().await;
        rm.initialize()
            .await
            .expect("Failed to initialize ResourceManager");
    }

    // Use the ResourceManager's TLK parser (includes Workshop TLK if available)
    let tlk_parser = {
        let rm = resource_manager.read().await;
        rm.get_tlk_parser().unwrap_or_else(|| {
            panic!("TLK parser not available after ResourceManager initialization");
        })
    };

    // Initialize GameDataLoader
    let mut loader = GameDataLoader::new(resource_manager.clone());

    loader
        .initialize(tlk_parser, false)
        .await
        .expect("Failed to initialize GameDataLoader");

    // Initialize ItemPropertyDecoder
    let decoder = ItemPropertyDecoder::new(resource_manager.clone());

    TestContext {
        loader,
        resource_manager,
        decoder,
        _temp_dir: temp,
    }
}

#[allow(dead_code)]
pub fn toolset_bridge_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent")
        .join("toolset-bridge/dist/toolset-bridge.exe")
}

#[allow(dead_code)]
pub fn nwn2_install_path() -> PathBuf {
    if let Ok(env_path) = std::env::var("NWN2_INSTALL") {
        return PathBuf::from(env_path);
    }
    PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/NWN2 Enhanced Edition")
}

/// Returns `true` when either the bridge binary or the NWN2 install is missing,
/// after writing a human-readable skip reason to stderr. Callers should early-return.
#[allow(dead_code)]
pub fn skip_if_toolset_prereqs_missing(
    bridge: &std::path::Path,
    install: &std::path::Path,
) -> bool {
    if !bridge.is_file() {
        eprintln!(
            "skipping: bridge not built at {} — run `dotnet build -c Release -p:Platform=x64` in toolset-bridge/",
            bridge.display()
        );
        return true;
    }
    if !install.is_dir() {
        eprintln!("skipping: NWN2 install not found at {}", install.display());
        return true;
    }
    false
}

#[allow(dead_code)]
pub fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        let name = entry.file_name();
        let dst_path = dst.join(name);

        if ty.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            std::fs::copy(&path, &dst_path)?;
        }
    }
    Ok(())
}
