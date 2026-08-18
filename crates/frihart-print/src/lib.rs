//! Print jobs. Local PostScript only. No cloud print.

#![forbid(unsafe_code)]

use frihart_core::Result;
use frihart_gfx::{DisplayList, DisplayOp};

#[derive(Clone, Debug)]
pub struct Job {
    pub title: String,
    pub list: DisplayList,
}

pub fn to_ps(job: &Job) -> Result<Vec<u8>> {
    let mut out = String::new();
    out.push_str("%!PS-Adobe-3.0\n");
    out.push_str(&format!("%%Title: {}\n", ps_escape(&job.title)));
    out.push_str("%%Pages: 1\n%%EndComments\n");
    out.push_str("/Courier findfont 12 scalefont setfont\n");
    let mut y = 760.0_f32;
    for op in &job.list.ops {
        match op {
            DisplayOp::Text { text, size, .. } => {
                let size = (*size).clamp(8.0, 32.0);
                out.push_str(&format!("{size} scalefont setfont\n"));
                out.push_str(&format!("72 {} moveto ({}) show\n", y, ps_escape(text)));
                y -= size * 1.4;
                if y < 72.0 {
                    break;
                }
            }
            DisplayOp::Fill { .. } | DisplayOp::Field { .. } => {}
        }
    }
    out.push_str("showpage\n%%EOF\n");
    Ok(out.into_bytes())
}

fn ps_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' | '\r' => ' '.to_string(),
            c if c.is_ascii_graphic() || c == ' ' => c.to_string(),
            _ => ' '.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_local_postscript() {
        let job = Job {
            title: "x".into(),
            list: DisplayList {
                ops: vec![DisplayOp::Text {
                    x: 0.0,
                    y: 0.0,
                    color: 0,
                    size: 16.0,
                    weight: 400,
                    text: "Hello".into(),
                    href: None,
                    max_width: 200.0,
                    wrap: true,
                }],
            },
        };
        let bytes = to_ps(&job).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("%!PS-Adobe-3.0"));
        assert!(s.contains("Hello"));
    }
}
