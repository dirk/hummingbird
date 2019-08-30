use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

use super::super::super::ast;
use super::read_and_parse_file;
use std::time::Duration;

struct File {
    module: ast::Module,
    source: String,
}

type FileError = String;

struct Reader {
    // Will be filled in once the load is complete.
    result: Option<Result<(ast::Module, String), FileError>>,
    pair: Arc<(Mutex<bool>, Condvar)>,
}

type Cache = Arc<Mutex<HashMap<PathBuf, Reader>>>;

/// Executes file reads in parallel.
pub struct FileReader {
    cache: Cache,
    sender: Sender<PathBuf>,
}

impl FileReader {
    pub fn new() -> Self {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = mpsc::channel();

        let movable = cache.clone();
        thread::spawn(move || FileReader::receive(receiver, movable));

        Self { cache, sender }
    }

    pub fn get(&self, path: PathBuf) -> Result<(ast::Module, String), FileError> {
        let is_cached_and_loaded = self.is_cached_and_loaded(&path);

        println!("is_cached_and_loaded {:?} {:?}", is_cached_and_loaded, &path);

        if !is_cached_and_loaded {
            // If there's no value then enqueue the read and wait until it is done.
            let pair = self.enqueue(path.clone());

            // Then wait on the condvar to be flipped by the background thread.
            let (mutex, condvar) = &*pair;
            let mut loaded = mutex.lock().unwrap();
            while !*loaded {
                loaded = condvar.wait(loaded).unwrap();
            }
        }

        let locked = self.cache.lock().unwrap();
        // Now try reading again.
        locked
            .get(&path)
            .and_then(|reader| reader.result.clone())
            .expect(&format!(
                "No entry in cache for path {:?}; this is a serious synchronization error",
                path
            ))
    }

    fn is_cached_and_loaded(&self, path: &PathBuf) -> bool {
        let locked = self.cache.lock().unwrap();
        let reader = match locked.get(path) {
            Some(reader) => reader,
            None => return false,
        };
        reader.result.is_some()
    }

    pub fn enqueue(&self, path: PathBuf) -> Arc<(Mutex<bool>, Condvar)> {
        let mut locked = self.cache.lock().unwrap();
        // First check to make sure no other thread snuck in a reader. If it
        // did clone the waiter and return that.
        if let Some(existing) = locked.get(&path) {
            return existing.pair.clone();
        }
        // Build a new reader, insert it into the cache, enqueue the work, and
        // return the mutex-condvar pair to wait until it's done.
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let reader = Reader {
            result: None,
            pair: pair.clone(),
        };
        locked.insert(path.clone(), reader);
        println!("send {:?}", path);
        self.sender.send(path).unwrap();
        pair
    }

    fn receive(receiver: Receiver<PathBuf>, cache: Cache) {
        loop {
            let path = match receiver.recv() {
                Ok(path) => path,
                Err(_) => return,
            };
            println!("receive {:?}", path);
            thread::sleep(Duration::from_millis(1));

            let result = FileReader::read_and_parse_file(path.as_path());

            let mut locked = cache.lock().unwrap();
            let reader = locked.get_mut(&path).expect(&format!(
                "No entry in cache for path {:?}; this is a serious synchronization error",
                path
            ));

            // Write the result into the reader.
            reader.result = Some(result);

            // And flip the condvar to notify the waiter(s) that it's done.
            let (mutex, condvar) = &*reader.pair;
            let mut loaded = mutex.lock().unwrap();
            *loaded = true;
            condvar.notify_all();

            println!("done {:?}", path);
        }
    }

    fn read_and_parse_file(path: &Path) -> Result<(ast::Module, String), FileError> {
        let result = read_and_parse_file(path);
        match result {
            Ok(value) => Ok(value),
            Err(error) => Err(format!("{:?}", error)),
        }
    }
}

impl FileReader {
    //    fn read_and<F, T>(&self, path: PathBuf, f: F) -> Result<T, VmError>
    //    where
    //        F: FnOnce(&File) -> Result<T, VmError>,
    //    {
    //        let cache = self.cache.read().unwrap();
    //        let result = cache.get(&path).unwrap();
    //        match result {
    //            Ok(file) => f(file),
    //            Err(error) => Err(error.clone()),
    //        }
    //    }
}

struct Eventual<T> {
    cell: Arc<Mutex<Option<T>>>,
    done: Arc<(Mutex<bool>, Condvar)>,
}

impl<T: Send + 'static> Eventual<T> {
    fn new<F>(f: F)
    where
        F: FnOnce() -> T + Send + 'static,
    {
        thread::spawn(|| f());
    }
}
