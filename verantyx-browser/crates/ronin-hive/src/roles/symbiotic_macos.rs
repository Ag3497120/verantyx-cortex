//! Symbiotic MacOS Integration Bridge
//! End-Game BotGuard Evasion: Zero-DOM Architecture
//! Implements absolute OS spatial tracking and CoreGraphics biometric mouse drift.

use tokio::process::Command;
use tracing::{info, warn};
use rand::Rng;

pub struct SafariBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub struct SymbioticMacOS;

impl SymbioticMacOS {
    /// Zero-DOM Phase 1: Retrieve the exact OS-level bounds of the frontmost Safari Window.
    pub async fn get_safari_bounds() -> Option<SafariBounds> {
        let script = r#"tell application "Safari" to get bounds of front window"#;
        let out = Command::new("osascript").arg("-e").arg(script).output().await.ok()?;
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Self::parse_bounds(&res)
    }

    /// Zero-DOM Phase 1B: Retrieve the OS-level bounds of the Verantyx custom stealth browser.
    pub async fn get_custom_browser_bounds() -> Option<SafariBounds> {
        let script = r#"
        tell application "System Events"
            repeat with p in (every process)
                try
                    set w to window "vx-agent-stealth" of p
                    if w exists then
                        set pos to position of w
                        set sz to size of w
                        return (item 1 of pos) & "," & (item 2 of pos) & "," & ((item 1 of pos) + (item 1 of sz)) & "," & ((item 2 of pos) + (item 2 of sz))
                    end if
                end try
            end repeat
            return ""
        end tell
        "#;
        let out = Command::new("osascript").arg("-e").arg(script).output().await.ok()?;
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Self::parse_bounds(&res)
    }

    fn parse_bounds(res: &str) -> Option<SafariBounds> {
        // Output format is typically "0, 25, 1440, 900" (x1, y1, x2, y2)
        let parts: Vec<&str> = res.split(',').collect();
        if parts.len() == 4 {
            let x1 = parts[0].trim().parse::<i32>().unwrap_or(0);
            let y1 = parts[1].trim().parse::<i32>().unwrap_or(0);
            let x2 = parts[2].trim().parse::<i32>().unwrap_or(0);
            let y2 = parts[3].trim().parse::<i32>().unwrap_or(0);
            if x2 > x1 && y2 > y1 {
                return Some(SafariBounds {
                    x: x1,
                    y: y1,
                    width: x2 - x1,
                    height: y2 - y1,
                });
            }
        }
        None
    }

    /// Zero-DOM Phase 2: Anchor Extraction.
    /// Injects a targeted script into Safari to read the exact geometric coordinates of the Blinking Text Cursor (Caret)
    /// representing the user's current spatial position.
    pub async fn get_caret_anchor_coordinates() -> Option<(f32, f32)> {
        let js = r#"
            let sel = window.getSelection();
            if (sel.rangeCount > 0) {
                let rect = sel.getRangeAt(0).getBoundingClientRect();
                rect.right + ',' + rect.bottom;
            } else {
                "0,0"
            }
        "#;
        let script = format!("tell application \"Safari\" to do JavaScript \"{}\" in front document", js);
        let out = Command::new("osascript").arg("-e").arg(script).output().await.ok()?;
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        
        let parts: Vec<&str> = res.split(',').collect();
        if parts.len() == 2 {
             let cx = parts[0].parse::<f32>().unwrap_or(0.0);
             let cy = parts[1].parse::<f32>().unwrap_or(0.0);
             if cx > 0.0 && cy > 0.0 {
                 return Some((cx, cy));
             }
        }
        None
    }

    /// Zero-DOM Phase 3: Biometric CoreGraphics Drift.
    /// Utilizes JXA (JavaScript for Automation) to natively invoke CoreGraphics C-bindings
    /// traversing the OS mouse across the screen with jitter matrices to evade trajectory analysis.
    pub async fn drift_mouse_to_target(start_x: f32, start_y: f32, end_x: f32, end_y: f32) -> anyhow::Result<()> {
        let (steps, base_delay, final_delay) = {
            let mut rng = rand::thread_rng();
            (
                rng.gen_range(20..40),
                rng.gen_range(8000..15000),
                rng.gen_range(100000..300000)
            )
        };
        
        // We write the JXA script dynamically
        let jxa_script = format!(
            r#"
            ObjC.import('CoreGraphics');
            ObjC.import('Foundation');

            var startX = {:.1};
            var startY = {:.1};
            var endX = {:.1};
            var endY = {:.1};
            var steps = {};
            var delayMs = {};

            for (var i = 0; i <= steps; i++) {{
                var t = i / steps;
                
                // Add Bezier/Sine Ease-in-out curve
                var ease_t = t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;

                // Add physical jitter
                var jitterX = (Math.random() - 0.5) * 4.0;
                var jitterY = (Math.random() - 0.5) * 4.0;
                if (i === steps) {{ jitterX = 0; jitterY = 0; }} // Precise snap on final frame
                
                var cx = startX + (endX - startX) * ease_t + jitterX;
                var cy = startY + (endY - startY) * ease_t + jitterY;
                
                var point = $.CGPointMake(cx, cy);
                var event = $.CGEventCreateMouseEvent(
                    null, 
                    $.kCGEventMouseMoved, 
                    point, 
                    0
                );
                $.CGEventPost($.kCGHIDEventTap, event);
                
                $.usleep(delayMs);
            }}
            
            // Wait random MS then click
            $.usleep({});
            var finalPoint = $.CGPointMake(endX, endY);
            var clickDown = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseDown, finalPoint, $.kCGMouseButtonLeft);
            var clickUp = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseUp, finalPoint, $.kCGMouseButtonLeft);
            $.CGEventPost($.kCGHIDEventTap, clickDown);
            $.usleep(Math.random() * 50000 + 40000); // Human click depress tension
            $.CGEventPost($.kCGHIDEventTap, clickUp);
            "#,
            start_x, start_y, end_x, end_y, steps, base_delay, final_delay
        );

        let script_path = std::env::temp_dir().join("symbiotic_drift.js");
        std::fs::write(&script_path, jxa_script)?;

        info!("[OS_BRIDGE] Engaging CoreGraphics Biometric Drift -> ({}, {})", end_x, end_y);
        let _ = Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg(script_path.to_str().unwrap())
            .output()
            .await?;
            
        Ok(())
    }

    /// Sets the macOS clipboard content using pbcopy.
    pub async fn set_clipboard(text: &str) -> anyhow::Result<()> {
        use std::process::Stdio;
        use tokio::io::AsyncWriteExt;
        
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
        }

        child.wait().await?;
        Ok(())
    }

    /// Gets the name of the currently active macOS application.
    pub async fn get_active_app() -> Option<String> {
        let script = r#"tell application "System Events" to get name of first application process whose frontmost is true"#;
        let out = Command::new("osascript").arg("-e").arg(script).output().await.ok()?;
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Some(res)
    }

    /// Forces a specific application to the foreground.
    pub async fn focus_app(app_name: &str) -> anyhow::Result<()> {
        let script = format!(r#"tell application "{}" to activate"#, app_name);
        Command::new("osascript").arg("-e").arg(&script).output().await?;
        Ok(())
    }
}
