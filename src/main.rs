use std::path::PathBuf;

fn main() {
    // KET_HOME is the .ket *home* directory; the CAS lives in its `cas/`
    // subdirectory, matching the ket CLI convention (`ket --home .ket`
    // opens `.ket/cas`). Opening the home dir directly looked for blobs at
    // `.ket/<cid>` instead of `.ket/cas/<cid>`, so a store created by the
    // ket CLI appeared empty (open succeeds — the dir exists — but every
    // get missed).
    let ket_home = std::env::var("KET_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".ket"));
    let cas_dir = ket_home.join("cas");

    let cas = match ket_cas::Store::open(cas_dir.clone()) {
        Ok(store) => store,
        Err(_) => {
            eprintln!("k-stack: initializing CAS at {}", cas_dir.display());
            ket_cas::Store::init(&cas_dir).unwrap_or_else(|e| {
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
