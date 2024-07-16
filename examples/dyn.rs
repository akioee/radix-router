use radix_router::{Meta, Router};

fn main() {
    let mut router = Router::new();

    router.insert("a/:name", Meta::default());
    println!("{:#?}", router.lookup("a/jack"));
}
