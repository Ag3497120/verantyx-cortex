//! Symbiotic MacOS Integration Bridge
//! End-Game BotGuard Evasion: Zero-DOM Architecture
//! Implements absolute OS spatial tracking and CoreGraphics biometric mouse drift.

use tokio::process::Command;
use tracing::info;
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

    /// Zero-DOM Phase 3: Biometric CoreGraphics Slide Path.
    /// Utilizes JXA to natively invoke CoreGraphics C-bindings.
    /// Traverses the mouse progressively across multiple waypoints starting securely from its CURRENT physical location.
    pub async fn drift_mouse_through_path(waypoints_str: &str) -> anyhow::Result<()> {
        let jxa_script = format!(
            r#"
            ObjC.import('CoreGraphics');
            ObjC.import('Foundation');
            ObjC.import('stdlib');

            // Eliminate 'teleportation' by starting exactly from the OS physical mouse pointer
            var currentPos = $.CGEventGetLocation($.CGEventCreate(null));
            
            var waypointsRaw = "{}";
            var waypoints = waypointsRaw.split(";").map(function(p) {{
                var coords = p.split(",");
                return {{x: parseFloat(coords[0]), y: parseFloat(coords[1])}};
            }});
            
            waypoints.unshift({{x: currentPos.x, y: currentPos.y}});
            
            var delayMs = 12000;
            var stepsPerSegment = 30;

            for (var w = 0; w < waypoints.length - 1; w++) {{
                var startPt = waypoints[w];
                var endPt = waypoints[w+1];
                
                for (var i = 1; i <= stepsPerSegment; i++) {{
                    var t = i / stepsPerSegment;
                    
                    var ease_t = t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t;
                    
                    var jitterX = (Math.random() - 0.5) * 3.0;
                    var jitterY = (Math.random() - 0.5) * 3.0;
                    if (i === stepsPerSegment) {{ jitterX = 0; jitterY = 0; }} 
                    
                    var cx = startPt.x + (endPt.x - startPt.x) * ease_t + jitterX;
                    var cy = startPt.y + (endPt.y - startPt.y) * ease_t + jitterY;
                    
                    var point = $.CGPointMake(cx, cy);
                    var event = $.CGEventCreateMouseEvent(
                        null, 
                        $.kCGEventMouseMoved, 
                        point, 
                        0
                    );
                    $.CGEventPost($.kCGHIDEventTap, event);
                    
                    delay((delayMs + (Math.random() * 5000)) / 1000000.0);
                }}
                
                // Human hesitation at node boundaries
                delay((Math.random() * 60000 + 40000) / 1000000.0);
            }}
            
            // Final destination -> Natural tension click
            delay(120000 / 1000000.0); 
            var finalPoint = $.CGPointMake(waypoints[waypoints.length - 1].x, waypoints[waypoints.length - 1].y);
            var clickDown = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseDown, finalPoint, $.kCGMouseButtonLeft);
            var clickUp = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseUp, finalPoint, $.kCGMouseButtonLeft);
            $.CGEventPost($.kCGHIDEventTap, clickDown);
            delay((Math.random() * 60000 + 30000) / 1000000.0);
            $.CGEventPost($.kCGHIDEventTap, clickUp);
            "#,
            waypoints_str
        );

        let script_path = std::env::temp_dir().join("symbiotic_drift.js");
        std::fs::write(&script_path, jxa_script)?;

        info!("[OS_BRIDGE] Engaging Multi-Waypoint Biometric Slide Path -> [{}]", waypoints_str);
        let out = Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg(script_path.to_str().unwrap())
            .output()
            .await?;
            
        if !out.status.success() {
            let err_msg = String::from_utf8_lossy(&out.stderr);
            println!("{} ❌ [FATAL] drift_mouse_through_path OSASCRIPT CRASHED:\n{}", console::style("[AUTO]").red(), err_msg);
            anyhow::bail!("JXA Script crashed: {}", err_msg);
        }
            
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

    /// Securely retrieves text from the macOS clipboard.
    pub async fn get_clipboard() -> anyhow::Result<String> {
        let out = Command::new("pbpaste").output().await?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
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

    /// Dynamically determines which of the 3 tiled Safari windows to focus based on their spatial bounds.
    pub async fn focus_safari_panel(position: &str) -> anyhow::Result<()> {
        let condition = match position {
            "left"   => "if xPos < 90 then",
            "middle" => "if xPos >= 90 and xPos < 190 then",
            "right"  => "if xPos >= 190 then",
            _        => "if xPos < 90 then",
        };
        
        let script = format!(r#"
            tell application "Safari"
                activate
                set winList to every window
                repeat with w in winList
                    try
                        set bnd to bounds of w
                        set xPos to item 1 of bnd
                        {}
                            set index of w to 1
                            exit repeat
                        end if
                    end try
                end repeat
            end tell
        "#, condition);

        Command::new("osascript").arg("-e").arg(&script).output().await?;
        Ok(())
    }

    /// Autonomously pastes clipboard content handling BotGuard without human intervention.
    /// Uses Legacy Return method (Fallback)
    pub async fn auto_paste_and_send() -> anyhow::Result<()> {
        let script = r#"
        tell application "System Events"
            -- Simulate Cmd+V (Paste)
            keystroke "v" using command down
            delay 0.5
            -- Simulate Return (Send to Gemini / LLM)
            keystroke return
        end tell
        "#;
        Command::new("osascript").arg("-e").arg(script).output().await?;
        Ok(())
    }

    /// Zero-DOM Phase 2: Visual Pointer Drop Calibration (Primary)
    /// Finds the send button by dropping visually from the edge of the active textarea 
    /// and scanning for the first "cursor: pointer" element (Pointing Hand).
    pub async fn auto_visual_calibrated_paste_and_send(_payload: &str) -> anyhow::Result<()> {
        info!("[OS_BRIDGE] Engaging Zero-DOM Visual Pointer-Drop Calibration...");
        
        let paste_script = r#"
        tell application "System Events"
            keystroke "v" using command down
            delay 1.0
        end tell
        "#;
        Command::new("osascript").arg("-e").arg(paste_script).output().await?;

        // Pure geometry + CSS Visual State. Zero classname/tag scraping!
        // We find the 'Send' button by identifying the right-most 'cursor: pointer' element
        // in the bottom-right bounding quadrant of the active text area.
        let calib_js = r#"
            let active = document.activeElement;
            let coords = "";
            if (active) {
                let box = active.getBoundingClientRect();
                
                // WPT 1 (I-Beam): Insertion Caret at the wrap line
                let aX = box.right - 10;
                let aY = box.bottom - 10;
                
                // Attempt to target the TRUE insertion pointer (Caret) position directly
                let sel = window.getSelection();
                if (sel && sel.rangeCount > 0) {
                    let rects = sel.getRangeAt(0).getClientRects();
                    if (rects.length > 0) {
                        let rect = rects[rects.length - 1]; // Use last line of the rect
                        if (rect.right > 0 && rect.right < window.innerWidth) {
                            aX = rect.right;
                            aY = rect.top + (rect.height / 2);
                        }
                    }
                }
                
                // WPT 2 (Destination): Deterministic Send Button Tracker
                let cX = aX;
                let cY = aY + 50; // Fallback slightly below caret if we completely fail
                
                let btns = Array.from(document.querySelectorAll('button, [role="button"]'));
                let sendBtn = null;
                
                // First pass: look for explicit Send/送信 labels or inner send icon text
                for (let b of btns) {
                    let r = b.getBoundingClientRect();
                    if (r.width > 0 && r.height > 0) {
                        let label = b.getAttribute('aria-label') || '';
                        let title = b.getAttribute('title') || '';
                        let inner = b.innerHTML || '';
                        if (
                            label.includes('送信') || label.includes('Send') || 
                            title.includes('送信') || title.includes('Send') || 
                            inner.includes('send')
                        ) {
                            sendBtn = b;
                            break;
                        }
                    }
                }
                
                // Second pass: Find the button closest to bottom-right of the lower half viewport
                if (!sendBtn) {
                    let maxScore = -1;
                    for (let b of btns) {
                        let r = b.getBoundingClientRect();
                        if (r.width > 0 && r.height > 0 && r.top > (window.innerHeight / 2)) {
                            let score = r.left + r.top; // Prefer bottom-right-most Elements
                            if (score > maxScore) {
                                maxScore = score;
                                sendBtn = b;
                            }
                        }
                    }
                }
                
                if (sendBtn) {
                    let r = sendBtn.getBoundingClientRect();
                    cX = r.left + (r.width / 2);
                    cY = r.top + (r.height / 2);
                }
                
                // Result: Slide elegantly and diagonally from Caret (A) straight to the Button (C)!
                // Return Viewport Coordinates + Inner Size to calculate Safari OS Toolbars
                coords = aX + "," + aY + ";" + cX + "," + cY + "|" + window.innerWidth + "," + window.innerHeight;
            }
            coords;
        "#;
        
        let measure_script = format!(r#"tell application "Safari" to do JavaScript "{}" in front document"#, calib_js.replace("\"", "\\\""));
        let out = Command::new("osascript").arg("-e").arg(&measure_script).output().await?;
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        
        if !res.is_empty() && res.contains("|") {
            let chunks: Vec<&str> = res.split('|').collect();
            let path_str = chunks[0];
            let size_str = chunks[1];
            
            let sizes: Vec<&str> = size_str.split(',').collect();
            let inner_w = sizes[0].parse::<f32>().unwrap_or(1000.0);
            let inner_h = sizes[1].parse::<f32>().unwrap_or(800.0);
            
            let bounds = Self::get_safari_bounds().await.unwrap_or(SafariBounds { x: 0, y: 0, width: 1440, height: 900 });
            
            // Safari Chrome Height mapping (Titlebar + Tab bar + Address bar height)
            let chrome_y = (bounds.height as f32 - inner_h).max(0.0);
            let chrome_x = (bounds.width as f32 - inner_w).max(0.0) / 2.0; // split left/right borders if any
            
            let mut os_waypoints = Vec::new();
            for pt in path_str.split(';') {
                let coords: Vec<&str> = pt.split(',').collect();
                if coords.len() == 2 {
                    let vx = coords[0].parse::<f32>().unwrap_or(0.0);
                    let vy = coords[1].parse::<f32>().unwrap_or(0.0);
                    
                    let os_x = bounds.x as f32 + chrome_x + vx;
                    let os_y = bounds.y as f32 + chrome_y + vy;
                    os_waypoints.push(format!("{:.1},{:.1}", os_x, os_y));
                }
            }
            
            let os_path_str = os_waypoints.join(";");
            info!("[OS_BRIDGE] Viewport -> OS Path Mapped: [{}]", os_path_str);
            let _ = Self::drift_mouse_through_path(&os_path_str).await;
            return Ok(());
        }

        info!("[OS_BRIDGE] Visual Calibration Missed Pointer. Forcing exact geometric drop-down...");
        
        // Geometric dropdown fallback utilizing the `=` boundary stabilization.
        // The send button is exactly at the bottom right corner of the Safari window bounds.
        let bounds = Self::get_safari_bounds().await.unwrap_or(SafariBounds { x: 0, y: 0, width: 1440, height: 900 });
        
        let base_x = bounds.x + bounds.width;
        let target_x = base_x as f32 - 50.0;
        
        // Send button is situated exactly above the bottom boundary of the Safari window
        let base_y = bounds.y + bounds.height;
        let target_y = base_y as f32 - 120.0; 

        info!("[OS_BRIDGE] Absolute geometric drop-down coordinates mapped -> ({}, {})", target_x, target_y);
        
        let _ = Self::drift_mouse_through_path(&format!("{:.1},{:.1}", target_x, target_y)).await;
        
        Ok(())
    }

    /// Autonomously grabs the full DOM output from the browser via Cmd+A and Cmd+C
    pub async fn auto_copy_all() -> anyhow::Result<()> {
        let script = r#"
        tell application "System Events"
            keystroke "a" using command down
            delay 0.5
            keystroke "c" using command down
            delay 0.5
        end tell
        "#;
        Command::new("osascript").arg("-e").arg(script).output().await?;
        Ok(())
    }

    /// Primary Extraction Method:
    /// Uses native Javascript DOM bounds detection to find the exact coordinates of the
    /// Gemini model's output "Copy" button and the input area. It then performs native OS clicks 
    /// to trigger the clean Markdown copy, returns to focus the input box, and clears it.
    pub async fn auto_visual_calibrated_extract_and_cleanup() -> anyhow::Result<()> {
        info!("[OS_BRIDGE] Engaging DOM Coordinate Extraction & Cleanup...");

        // Step 1: Force scroll to absolute bottom to ensure elements are visibly mounted in DOM tree
        let scroll_script = r#"
        tell application "System Events"
            key code 125 using command down
            delay 0.8
        end tell
        "#;
        Command::new("osascript").arg("-e").arg(scroll_script).output().await?;

        // Step 2: Extract absolute viewport coordinates via DOM inspection
        let dom_inspector_js = r#"
            let cX = 0; let cY = 0; // Copy Button Coordinates
            let aX = 0; let aY = 0; // Input Box Coordinates
            
            // Task A: Locate the "Copy" tooltip button of the *last* AI response
            let btns = Array.from(document.querySelectorAll('button, [role="button"], [data-test-id="copy-button"]'));
            let copyBtns = btns.filter(b => {
                let label = b.getAttribute('aria-label') || '';
                let title = b.getAttribute('title') || '';
                let mat = b.getAttribute('mattooltip') || '';
                let testId = b.getAttribute('data-test-id') || '';
                let inner = b.innerHTML || '';
                return label.includes('コピー') || label.includes('Copy') || 
                       title.includes('コピー') || title.includes('Copy') ||
                       mat.includes('コピー') || mat.includes('Copy') ||
                       testId.includes('copy') || inner.includes('content_copy');
            });
            
            if (copyBtns.length > 0) {
                // The last valid copy button on the page is typically the one for the most recent response
                let target = copyBtns[copyBtns.length - 1];
                let rect = target.getBoundingClientRect();
                cX = rect.left + (rect.width / 2);
                cY = rect.top + (rect.height / 2);
            } else {
                // Heuristic Fallback: Bottom-right of the last chat container
                let msgs = Array.from(document.querySelectorAll('message-content, .message-content, [data-test-id="response-message"]'));
                if (msgs.length > 0) {
                    let lastMsg = msgs[msgs.length - 1];
                    let rect = lastMsg.getBoundingClientRect();
                    cX = rect.right - 20; 
                    cY = rect.bottom + 20; 
                } else {
                    cX = window.innerWidth / 2;
                    cY = window.innerHeight - 200;
                }
            }

            // Task B: Locate the main Input Box (to focus and clean it up)
            let inputs = Array.from(document.querySelectorAll('div[contenteditable="true"], textarea, rich-textarea'));
            if (inputs.length > 0) {
                let target = inputs[inputs.length - 1];
                let rect = target.getBoundingClientRect();
                aX = rect.left + (rect.width / 2);
                aY = rect.top + (rect.height / 2);
            } else {
                aX = window.innerWidth / 2;
                aY = window.innerHeight - 50; 
            }
            
            cX + "," + cY + ";" + aX + "," + aY + "|" + window.innerWidth + "," + window.innerHeight;
        "#;
        
        let measure_script = format!(r#"tell application "Safari" to do JavaScript "{}" in front document"#, dom_inspector_js.replace("\"", "\\\""));
        let jxa_res = match Command::new("osascript").arg("-e").arg(&measure_script).output().await {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            Err(_) => String::new(),
        };

        if !jxa_res.is_empty() && jxa_res.contains("|") {
            let chunks: Vec<&str> = jxa_res.split('|').collect();
            let path_str = chunks[0];
            let size_str = chunks[1];

            let sizes: Vec<&str> = size_str.split(',').collect();
            let inner_w = sizes[0].parse::<f32>().unwrap_or(1000.0);
            let inner_h = sizes[1].parse::<f32>().unwrap_or(800.0);

            let bounds = Self::get_safari_bounds().await.unwrap_or(SafariBounds { x: 0, y: 0, width: 1440, height: 900 });
            let chrome_y = (bounds.height as f32 - inner_h).max(0.0);
            let chrome_x = (bounds.width as f32 - inner_w).max(0.0) / 2.0; 
            
            let mut os_waypoints = Vec::new();
            for pt in path_str.split(';') {
                let coords: Vec<&str> = pt.split(',').collect();
                if coords.len() == 2 {
                    let vx = coords[0].parse::<f32>().unwrap_or(0.0);
                    let vy = coords[1].parse::<f32>().unwrap_or(0.0);
                    
                    let os_x = bounds.x as f32 + chrome_x + vx;
                    let os_y = bounds.y as f32 + chrome_y + vy;
                    os_waypoints.push(format!("{:.1},{:.1}", os_x, os_y));
                }
            }

            // Step 3: Perform OS extraction dance C (Copy), then A (Input Focus & Clean)
            if os_waypoints.len() == 2 {
                let c_pos = &os_waypoints[0];
                let a_pos = &os_waypoints[1];
                
                info!("[OS_BRIDGE] Commencing DOM Coordinate Extraction Path -> C:({}), A:({})", c_pos, a_pos);

                let inject_script = format!(
                    r#"
                    var SystemEvents = Application('System Events');
                    var delay = function(sec) {{ $.usleep(sec * 1000000); }};
                    
                    // 1. Move to Point C (Copy Button)
                    var cParts = "{}".split(",");
                    var copyPoint = $.CGPointMake(parseFloat(cParts[0]), parseFloat(cParts[1]));
                    
                    var slideToC = $.CGEventCreateMouseEvent(null, $.kCGEventMouseMoved, copyPoint, $.kCGMouseButtonLeft);
                    $.CGEventPost($.kCGHIDEventTap, slideToC);
                    delay(0.2); // Hover delay for tooltip
                    
                    var clickDownC = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseDown, copyPoint, $.kCGMouseButtonLeft);
                    var clickUpC = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseUp, copyPoint, $.kCGMouseButtonLeft);
                    $.CGEventPost($.kCGHIDEventTap, clickDownC);
                    delay(0.05);
                    $.CGEventPost($.kCGHIDEventTap, clickUpC);
                    delay(0.8); // Wait for Gemini to copy to clipboard cleanly
                    
                    // 2. Move to Point A (Input Box)
                    var aParts = "{}".split(",");
                    var inputPoint = $.CGPointMake(parseFloat(aParts[0]), parseFloat(aParts[1]));
                    
                    var slideToA = $.CGEventCreateMouseEvent(null, $.kCGEventMouseMoved, inputPoint, $.kCGMouseButtonLeft);
                    $.CGEventPost($.kCGHIDEventTap, slideToA);
                    delay(0.2);
                    
                    var clickDownA = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseDown, inputPoint, $.kCGMouseButtonLeft);
                    var clickUpA = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseUp, inputPoint, $.kCGMouseButtonLeft);
                    $.CGEventPost($.kCGHIDEventTap, clickDownA);
                    delay(0.05);
                    $.CGEventPost($.kCGHIDEventTap, clickUpA);
                    delay(0.2);
                    "#,
                    c_pos, a_pos
                );

                let script_path = std::env::temp_dir().join("symbiotic_extract.js");
                std::fs::write(&script_path, inject_script)?;
                let _ = Command::new("osascript").arg("-l").arg("JavaScript").arg(script_path.to_str().unwrap()).output().await?;

                // 4. Input Box Clear (Cmd+A -> Delete)
                let cleanup_script = r#"
                tell application "System Events"
                    keystroke "a" using command down
                    delay 0.2
                    key code 51 -- delete key
                    delay 0.2
                end tell
                "#;
                Command::new("osascript").arg("-e").arg(cleanup_script).output().await?;
                
                info!("[OS_BRIDGE] DOM Element Extraction & Cleanup successfully executed.");
                return Ok(());
            }
        }
        
        info!("[OS_BRIDGE] Extraction failed definitively. Could not parse JS coordinates.");
        // We do NOT use auto_copy_all anymore.
        anyhow::bail!("DOM Extraction Failed.");
    }
}
