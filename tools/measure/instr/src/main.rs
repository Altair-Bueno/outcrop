use wasmtime::Config;
use wasmtime::Engine;
use wasmtime::Linker;
use wasmtime::Module;
use wasmtime::Store;

const SCENARIOS: [&str; 16] = [
    "builder_idle",
    "builder_feed_bottom",
    "builder_feed_unrelated",
    "builder_feed_top",
    "builder_feed_flat",
    "builder_route",
    "macro_idle",
    "macro_feed_bottom",
    "macro_feed_unrelated",
    "macro_feed_top",
    "macro_feed_flat",
    "macro_route",
    "builder_bank_one",
    "builder_bank_all",
    "macro_bank_one",
    "macro_bank_all",
];

fn run(engine: &Engine, module: &Module, name: &str, rounds: u32) -> Result<(u64, u32), Box<dyn std::error::Error>> {
    let mut store = Store::new(engine, ());
    store.set_fuel(1 << 40)?;
    let instance = Linker::<()>::new(engine).instantiate(&mut store, module)?;
    let func = instance.get_typed_func::<u32, u32>(&mut store, name)?;
    let before = store.get_fuel()?;
    let reports = func.call(&mut store, rounds)?;
    Ok((before - store.get_fuel()?, reports))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: instr <workload.wasm>")?;
    let bytes = std::fs::read(path)?;
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &bytes[..])?;
    println!("  scenario\tfuel/round\treports(16 rounds)");
    for name in SCENARIOS {
        run(&engine, &module, name, 1)?;
        let (short, reports) = run(&engine, &module, name, 16)?;
        let (long, _) = run(&engine, &module, name, 32)?;
        println!("  {name}\t{}\t{reports}", (long - short) / 16);
    }
    Ok(())
}
