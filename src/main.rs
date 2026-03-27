use std::path::PathBuf;

fn main() {
    let ket_home = std::env::var("KET_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".ket"));

    let cas = match ket_cas::Store::open(ket_home.clone()) {
        Ok(store) => store,
        Err(_) => {
            eprintln!("k-stack: initializing CAS at {}", ket_home.display());
            ket_cas::Store::init(&ket_home).unwrap_or_else(|e| {
                eprintln!("k-stack: failed to init CAS: {e}");
                std::process::exit(1);
            })
        }
    };

    if let Err(e) = k_stack::run_stdio_server(&cas) {
        eprintln!("k-stack: {e}");
        std::process::exit(1);
    }
}
