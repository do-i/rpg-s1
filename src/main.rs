fn main() -> std::process::ExitCode {
    rpg_engine::run_with_game_version(env!("CARGO_PKG_VERSION"))
}
