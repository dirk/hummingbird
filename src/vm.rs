// use async_recursion::async_recursion;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

/// Index of the module in the loader's `modules` vec.
type ModuleId = usize;

type Vm = Arc<RawVm>;

#[derive(Debug)]
pub struct RawVm {
    loader: Loader,
}

pub fn new(root: &'static str) -> Vm {
    let canonical = Path::new(root).canonicalize().unwrap();
    println!("root: {:?}", canonical);
    Arc::new(RawVm {
        loader: Loader {
            root: canonical,
            table: RwLock::new(ModuleTable::new()),
        },
    })
}

pub async fn run(vm: Vm, entry_path: &'static str) {
    let mut epoch = Epoch::new();
    let entry_gen = vm.loader.load(&mut epoch, entry_path).await;
    println!("epoch: {:#?}", epoch);
}

#[derive(Debug)]
struct Epoch {
    modules: Vec<Option<ModGen>>,
}

impl Epoch {
    fn new() -> Self {
        Self {
            modules: vec![],
        }
    }
}

#[derive(Debug)]
struct Loader {
    // TODO: Implement resolvers rather than just hardcoding a root.
    root: PathBuf,
    table: RwLock<ModuleTable>,
}

impl Loader {
    /// Load a module and all of its dependencies into the current epoch,
    /// reusing existing generations if possible.
    /// 
    /// TODO: Support epoch policies to allow control of checking mtimes and
    /// checksums to determine when modules need to be reloaded.
    async fn load(&self, epoch: &mut Epoch, path: &str) -> ModGen {
        let module = self.get_or_init(path);

        // If we've already loaded this module in this epoch then return it.
        if let Some(option) = epoch.modules.get(module.id) {
            if let Some(gen) = option {
                return gen.clone();
            }
        }

        // As we're about to (re)load the module into this epoch, ensure we
        // have space for it.
        if module.id >= epoch.modules.capacity() {
            epoch.modules.resize(module.id + 1, None);
        }

        let metadata = fs::metadata(module.canonical.clone()).await.unwrap();
        let mtime = metadata.modified().unwrap();
        if let Some(gen) = module.gen_matching_mtime(mtime) {
            epoch.modules[module.id] = Some(gen.clone());
            return gen;
        }

        let mut file = File::open(path).await.unwrap();
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await.unwrap();

        let checksum = u32::checksum_from_str(&buffer);
        if let Some(gen) = module.gen_matching_checksum(checksum) {
            epoch.modules[module.id] = Some(gen.clone());
            return gen;
        }

        let gen = module.push_gen(mtime, checksum);
        epoch.modules[module.id] = Some(gen.clone());
        gen
    }

    /// Canonicalizes the path and returns an existing or empty module for that path.
    fn get_or_init(&self, path: &str) -> Module {
        let canonical = self.root.join(path).canonicalize().unwrap();
        {
            let table = self.table.read();
            if let Some(id) = table.canonical_to_id.get(&canonical) {
                return table.modules[*id].clone();
            }
        }
        let mut table = self.table.write();
        let module = Arc::new(RawModule::new(table.modules.len(), canonical.clone()));
        table.modules.push(module.clone());
        table.canonical_to_id.insert(canonical, module.id);
        module
    }
}

#[derive(Debug)]
struct ModuleTable {
    modules: Vec<Module>,
    canonical_to_id: HashMap<PathBuf, ModuleId>,
}

impl ModuleTable {
    fn new() -> Self {
        Self {
            modules: vec![],
            canonical_to_id: HashMap::new(),
        }
    }
}

type Module = Arc<RawModule>;

#[derive(Debug)]
struct RawModule {
    id: ModuleId,
    canonical: PathBuf,
    generations: RwLock<Vec<Gen>>,
}

impl RawModule {
    fn new(id: ModuleId, canonical: PathBuf) -> Self {
        Self {
            id,
            canonical,
            generations: RwLock::new(vec![]),
        }
    }

    fn gen_matching_mtime(&self, mtime: SystemTime) -> Option<ModGen> {
        for gen in self.generations.read().iter() {
            if let Gen::Alive(gen) = gen {
                if gen.mtime == mtime {
                    return Some(gen.clone());
                }
            }
        }
        None
    }

    fn gen_matching_checksum(&self, checksum: u32) -> Option<ModGen> {
        for gen in self.generations.read().iter() {
            if let Gen::Alive(gen) = gen {
                if gen.checksum == checksum {
                    return Some(gen.clone());
                }
            }
        }
        None
    }

    fn push_gen(&self, mtime: SystemTime, checksum: u32) -> ModGen {
        let id = self.generations.read().len();
        let gen = Arc::new(RawModGen {
            id,
            mtime,
            checksum,
        });
        self.generations.write().push(Gen::Alive(gen.clone()));
        gen
    }
}

/// Index of the generation in the module's `generations` vec.
type GenId = usize;

#[derive(Debug)]
enum Gen {
    Dead(GenId),
    Alive(ModGen),
}

type ModGen = Arc<RawModGen>;

#[derive(Debug)]
struct RawModGen {
    id: GenId,
    mtime: SystemTime,
    checksum: u32,
}

#[derive(Clone)]
struct GenRef {
    module_id: ModuleId,
    gen: ModGen,
}

pub trait ShortChecksum {
    fn checksum_from_str(input: &str) -> Self;
}

impl ShortChecksum for u32 {
    fn checksum_from_str(input: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        u32::from_ne_bytes(digest[0..4].try_into().unwrap())
    }
}
