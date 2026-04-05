//! Terminal splash banner and status display utilities.

use console::style;

pub fn print_banner() {
    println!();
    println!("{}", style("██████╗  ██████╗ ███╗   ██╗██╗███╗   ██╗").cyan().bold());
    println!("{}", style("██╔══██╗██╔═══██╗████╗  ██║██║████╗  ██║").cyan().bold());
    println!("{}", style("██████╔╝██║   ██║██╔██╗ ██║██║██╔██╗ ██║").cyan().bold());
    println!("{}", style("██╔══██╗██║   ██║██║╚██╗██║██║██║╚██╗██║").cyan().dim());
    println!("{}", style("██║  ██║╚██████╔╝██║ ╚████║██║██║ ╚████║").cyan().dim());
    println!("{}", style("╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚═╝╚═╝  ╚═══╝").cyan().dim());
    println!();
    println!(
        "  {} {} {}",
        style("🐺 Autonomous Hacker Agent").bold(),
        style("·").dim(),
        style("Local-First · Memory-Native · Policy-Safe").dim()
    );
    println!();
}

pub fn print_config_summary(model: &str, hitl: bool, lang: &str, steps: u32) {
    println!("{}", style("─".repeat(56)).dim());
    println!(
        "  {:<14} {}",
        style("Model:").dim(),
        style(model).green().bold()
    );
    println!(
        "  {:<14} {}",
        style("HITL:").dim(),
        if hitl { style("enabled").green() } else { style("disabled").yellow() }
    );
    println!(
        "  {:<14} {}",
        style("Language:").dim(),
        style(lang).cyan()
    );
    println!(
        "  {:<14} {}",
        style("Max Steps:").dim(),
        style(steps.to_string()).white()
    );
    println!("{}", style("─".repeat(56)).dim());
    println!();
}

pub fn print_step_header(step: u32, total: u32, description: &str) {
    println!(
        "\n{} {} {}",
        style(format!("[{}/{}]", step, total)).cyan().bold(),
        style("▶").green(),
        style(description).bold()
    );
}

pub fn print_observation(observation: &str) {
    println!();
    println!("{}", style("╔═ OBSERVATION ══════════════════════════════════").dim());
    for line in observation.lines().take(40) {
        println!("{} {}", style("║").dim(), line);
    }
    println!("{}", style("╚════════════════════════════════════════════════").dim());
}

pub fn print_success(message: &str) {
    println!("\n{} {}", style("✅").green(), style(message).bold());
}

pub fn print_warning(message: &str) {
    println!("\n{} {}", style("⚠️ ").yellow(), style(message).yellow());
}

pub fn print_error(message: &str) {
    println!("\n{} {}", style("❌").red(), style(message).red().bold());
}
