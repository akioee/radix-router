use radix_router::{Meta, Router};

fn main() {
    let mut router = Router::new();
    let mut meta = Meta::default();

    meta.insert("name", "test route".into());
    router.insert("a/b", meta);
    println!("{:#?}", router.lookup("a/b"));
}
