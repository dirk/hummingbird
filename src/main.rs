use tokio::sync::OnceCell;

mod parser;
mod vm;

use vm::ShortChecksum;

async fn foo() -> u32 {
    println!("there");
    69
}

#[tokio::main]
async fn main() {
    // let cell = OnceCell::<u32>::new();
    // println!("{}", cell.get_or_init(|| async { 42 }).await);
    // println!("{}", cell.get_or_init(foo).await);
    // println!("{}", u32::checksum_from_str("test"));
    let vm = vm::new(".");
    vm::run(vm.clone(), "hello.hb").await;
    vm::run(vm.clone(), "hello.hb").await;
    // println!("{:#?}", vm);
}
