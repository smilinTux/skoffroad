// ASCII logo printed to stdout at app startup. Pure decoration; runs once.
//
// Public API:
//   AsciiLogoPlugin

use bevy::prelude::*;

pub struct AsciiLogoPlugin;

impl Plugin for AsciiLogoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, print_logo);
    }
}

fn print_logo() {
    println!("{}", BANNER);
    println!("  Press any key when ready...");
    println!();
    println!("  Quick keys: T=time-trial  R=race  P=pursuit  X=demo  C=challenge");
    println!("              Tab=map-select  H=help  Esc=menu");
    println!();
}

const BANNER: &str = r#"
╔══════════════════════════════════════════════════╗
║                                                  ║
║   ███████╗ █████╗ ███╗   ██╗██████╗ ██╗  ██╗   ║
║   ██╔════╝██╔══██╗████╗  ██║██╔══██╗██║ ██╔╝   ║
║   ███████╗███████║██╔██╗ ██║██║  ██║█████╔╝    ║
║   ╚════██║██╔══██║██║╚██╗██║██║  ██║██╔═██╗    ║
║   ███████║██║  ██║██║ ╚████║██████╔╝██║  ██╗   ║
║   ╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═════╝ ╚═╝  ╚═╝   ║
║                                                  ║
║            O F F R O A D   v0.5                  ║
║                                                  ║
║   Procedural off-road sandbox                    ║
║   Built with Claude Code + Bevy 0.18             ║
║                                                  ║
╚══════════════════════════════════════════════════╝"#;
