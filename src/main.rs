use std::io::stdout;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    cursor::{Hide, Show, MoveTo},
    event::{poll, read, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    style::Print,
};
use glam::{Vec3, Mat3};

mod canvas;
mod pdb;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input PDB file
    input: String,

    /// Size of the viewing box
    #[arg(short = 's', default_value_t = 100.0)]
    size: f32,

    /// Initial model to show (1-based)
    #[arg(short = 'm', default_value_t = 1)]
    model: usize,

    /// Chain to show
    #[arg(short = 'c', long = "chain")]
    chain: Option<String>,

    /// Format of the input file
    #[arg(short = 'f', long = "format")]
    format: Option<String>,
}

// Constants from Python version
const ZOOM_SPEED: f32 = 1.1;
const TRANS_SPEED: f32 = 1.0;
const ROT_SPEED: f32 = 0.1;
const SPIN_SPEED: f32 = 0.01;

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate box size
    if args.size < 10.0 || args.size > 400.0 {
        eprintln!("Box size must be between 10 and 400");
        return Ok(());
    }

    let models = pdb::read_pdb(&args.input, args.chain.as_deref(), args.format.as_deref()).context("Failed to read PDB file")?;

    // Current state
    let mut curr_model_idx = if args.model > 0 && args.model <= models.len() {
        args.model - 1
    } else {
        0
    };

    // Calculate bounding box and zoom (based on FIRST model, or current?)
    // Python code uses `coords[curr_model - 1]` initially (which is passed as arg to view).
    // Let's use the initial current model.
    let model0 = &models[curr_model_idx];

    let (min_pt, max_pt) = get_bounds(model0);
    let x_diff = max_pt.x - min_pt.x;
    let y_diff = max_pt.y - min_pt.y;
    let box_bound = x_diff.max(y_diff) + 2.0;

    let mut zoom = args.size / box_bound;

    // Clipping bounds calculation (Python logic replication)
    // x_min = zoom * (x_min - (box_bound - x_diff) / 2.0)
    // This seems to center the view box around the model?
    // Let's stick to the clipping window logic:
    // The "window" is defined by these bounds.
    let clip_x_min = zoom * (min_pt.x - (box_bound - x_diff) / 2.0);
    let clip_x_max = zoom * (max_pt.x + (box_bound - x_diff) / 2.0);
    let clip_y_min = zoom * (min_pt.y - (box_bound - y_diff) / 2.0);
    let clip_y_max = zoom * (max_pt.y + (box_bound - y_diff) / 2.0);

    let mut trans_x = 0.0;
    let mut trans_y = 0.0;
    let mut rot_x = 0.0;
    let mut rot_y = 0.0;

    let mut auto_spin = false;
    let mut cycle_models = false;
    let mut do_update = true;

    // Helper strings
    let help_str = "W/A/S/D rotates, T/F/G/H moves, I/O zooms, U spins, P cycles models, Q quits";

    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let mut canvas = canvas::Canvas::new();

    // Main Loop
    loop {
        if poll(Duration::from_millis(50))? {
            match read()? {
                Event::Key(KeyEvent { code, .. }) => {
                    do_update = true;
                    match code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => break,
                        KeyCode::Char('o') | KeyCode::Char('O') => zoom /= ZOOM_SPEED,
                        KeyCode::Char('i') | KeyCode::Char('I') => zoom *= ZOOM_SPEED,
                        KeyCode::Char('f') | KeyCode::Char('F') => trans_x -= TRANS_SPEED,
                        KeyCode::Char('h') | KeyCode::Char('H') => trans_x += TRANS_SPEED,
                        KeyCode::Char('g') | KeyCode::Char('G') => trans_y -= TRANS_SPEED,
                        KeyCode::Char('t') | KeyCode::Char('T') => trans_y += TRANS_SPEED,
                        KeyCode::Char('s') | KeyCode::Char('S') => rot_x -= ROT_SPEED,
                        KeyCode::Char('w') | KeyCode::Char('W') => rot_x += ROT_SPEED,
                        KeyCode::Char('a') | KeyCode::Char('A') => rot_y -= ROT_SPEED,
                        KeyCode::Char('d') | KeyCode::Char('D') => rot_y += ROT_SPEED,
                        KeyCode::Char('u') | KeyCode::Char('U') => auto_spin = !auto_spin,
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            if models.len() > 1 {
                                cycle_models = !cycle_models;
                            }
                        }
                        KeyCode::Esc => break,
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if auto_spin {
            rot_y += SPIN_SPEED;
            do_update = true;
        }

        if cycle_models {
            curr_model_idx = (curr_model_idx + 1) % models.len();
            do_update = true;
        }

        if do_update {
            do_update = false;
            canvas.clear();

            // Draw bounding box (lines between corners of the clip window)
            // Python:
            // (x_min, y_min) -> (x_max, y_min)
            // (x_max, y_min) -> (x_max, y_max)
            // ...
            // The python code draws the clipping rectangle itself on the canvas.
            let corners = [
                (clip_x_min, clip_y_min),
                (clip_x_max, clip_y_min),
                (clip_x_max, clip_y_max),
                (clip_x_min, clip_y_max),
            ];
            for i in 0..4 {
                let (x1, y1) = corners[i];
                let (x2, y2) = corners[(i + 1) % 4];
                canvas.line(x1, y1, x2, y2);
            }

            // Transform and Draw Atoms
            let model = &models[curr_model_idx];

            // Info String update
            let chain_ids: std::collections::HashSet<char> = model.atoms.iter().map(|a| a.chain_id).collect();
            let mut chains_sorted: Vec<char> = chain_ids.into_iter().collect();
            chains_sorted.sort();
            let chain_str: String = chains_sorted.into_iter().collect();

            let info_str = format!(
                "{} with {} models, {} chains ({}), {} atoms.",
                args.input, models.len(), chain_str.len(), chain_str, model.atoms.len()
            );

            // Calculate matrices
            let rot_mat_x = Mat3::from_rotation_x(rot_x);
            let rot_mat_y = Mat3::from_rotation_y(rot_y);
            // Combined rotation: Y * X (based on python: np.matmul(rot_mat_y, np.matmul(rot_mat_x, ...)))
            // In glam, Mat3 * Vec3 applies matrix.
            // Python: Ry * (Rx * v). So v' = Ry * Rx * v.
            // glam: rot_mat_y * (rot_mat_x * v).

            let translation = Vec3::new(trans_x, trans_y, 0.0);

            // Pre-calculate transformed points to avoid recalculating for connections
            let transformed_points: Vec<Vec3> = model.atoms.iter().map(|atom| {
                let p = atom.pos + translation; // Translate first (object space)
                let p = rot_mat_x * p;
                let p = rot_mat_y * p;
                p * zoom
            }).collect();

            // Draw connections
            for i in 0..model.connections.len() {
                if model.connections[i] {
                    let p1 = transformed_points[i];
                    let p2 = transformed_points[i+1];

                    // Check clipping
                    // Python: if x_min < x_start < x_max ...
                    if p1.x > clip_x_min && p1.x < clip_x_max &&
                       p1.y > clip_y_min && p1.y < clip_y_max &&
                       p2.x > clip_x_min && p2.x < clip_x_max &&
                       p2.y > clip_y_min && p2.y < clip_y_max {
                           canvas.line(p1.x, p1.y, p2.x, p2.y);
                       }
                }
            }

            // Render
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            execute!(stdout, Print(format!("{}\r\n", info_str)))?;
            execute!(stdout, Print(format!("{}\r\n", help_str)))?;
            execute!(stdout, MoveTo(0, 2))?; // Canvas starts below info
            execute!(stdout, Print(canvas.frame()))?;
        }
    }

    // Cleanup
    execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

fn get_bounds(model: &pdb::Model) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    if model.atoms.is_empty() {
        return (Vec3::ZERO, Vec3::ZERO);
    }

    for atom in &model.atoms {
        min = min.min(atom.pos);
        max = max.max(atom.pos);
    }
    (min, max)
}
