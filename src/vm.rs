// use async_recursion::async_recursion;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio;
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;

/// Index of the module in the loader's `modules` vec.
type ModuleId = usize;

type Vm = Arc<RawVm>;

#[derive(Debug)]
pub struct RawVm {
    resolver: Resolver,
}

pub fn new(root: &'static str) -> Vm {
    let canonical = Path::new(root).canonicalize().unwrap();
    println!("root: {:?}", canonical);
    Arc::new(RawVm {
        resolver: Resolver {
            root: canonical,
            table: RwLock::new(ModuleTable::new()),
        },
    })
}

pub async fn run(vm: Vm, entry_path: &'static str) {
    let mut epoch = Epoch::new();
    let entry_gen = epoch.load(&vm.resolver, entry_path).await;
    println!("epoch: {:#?}", epoch);
}

#[derive(Debug)]
struct Epoch {
    modules: Vec<Option<ModGen>>,
}

impl Epoch {
    fn new() -> Self {
        Self { modules: vec![] }
    }

    /// Load a module and all of its dependencies into the current epoch,
    /// reusing existing generations if possible.
    ///
    /// TODO: Support epoch policies to allow control of checking mtimes and
    /// checksums to determine when modules need to be reloaded.
    async fn load(&mut self, loader: &Resolver, path: &str) -> ModGen {
        let module = loader.get_or_init(path).await;

        // If we've already loaded this module in this epoch then return it.
        if let Some(option) = self.modules.get(module.id) {
            if let Some(gen) = option {
                return gen.clone();
            }
        }

        // As we're about to (re)load the module into this epoch, ensure we
        // have space for it.
        if module.id >= self.modules.capacity() {
            self.modules.resize(module.id + 1, None);
        }

        // Look for matching generations based on the mtime and checksum. Don't
        // need to acquire a lock around all of this because mtimes and
        // checksums are read-only and determinstic (famous last words).

        let metadata = fs::metadata(module.canonical.clone()).await.unwrap();
        let mtime = metadata.modified().unwrap();
        if let Some(gen) = module.gen_matching_mtime(mtime).await {
            self.modules[module.id] = Some(gen.clone());
            return gen;
        }

        let mut file = File::open(path).await.unwrap();
        let mut source = String::new();
        file.read_to_string(&mut source).await.unwrap();

        let checksum = u32::checksum_from_str(&source);
        if let Some(gen) = module.gen_matching_checksum(checksum).await {
            self.modules[module.id] = Some(gen.clone());
            return gen;
        }

        let gen = module.push_gen(mtime, checksum, source).await;
        self.modules[module.id] = Some(gen.clone());
        gen
    }
}

/// Responsible for resolving a path to a source file into a module ID. For the duration of the
/// VM's lifetime the resolver guarantees that the same file will always resolve to the same
/// module ID.
#[derive(Debug)]
struct Resolver {
    // TODO: Implement resolvers rather than just hardcoding a root.
    root: PathBuf,
    table: RwLock<ModuleTable>,
}

impl Resolver {
    /// Canonicalizes the path and returns an existing or empty module for that path.
    async fn get_or_init(&self, path: &str) -> Module {
        let canonical = self.root.join(path).canonicalize().unwrap();
        {
            let table = self.table.read().await;
            if let Some(id) = table.canonical_to_id.get(&canonical) {
                return table.modules[*id].clone();
            }
        }
        let mut table = self.table.write().await;
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

    async fn gen_matching_mtime(&self, mtime: SystemTime) -> Option<ModGen> {
        for gen in self.generations.read().await.iter() {
            if let Gen::Alive(gen) = gen {
                if gen.mtime == mtime {
                    return Some(gen.clone());
                }
            }
        }
        None
    }

    async fn gen_matching_checksum(&self, checksum: u32) -> Option<ModGen> {
        for gen in self.generations.read().await.iter() {
            if let Gen::Alive(gen) = gen {
                if gen.checksum == checksum {
                    return Some(gen.clone());
                }
            }
        }
        None
    }

    async fn push_gen(&self, mtime: SystemTime, checksum: u32, source: String) -> ModGen {
        let id = self.generations.read().await.len();
        let gen = Arc::new(RawModGen {
            id,
            mtime,
            checksum,
            source,
        });
        self.generations.write().await.push(Gen::Alive(gen.clone()));
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
    source: String,
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
