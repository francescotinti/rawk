/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: Lifecycle degli stream I/O del runtime — apertura/append file
 *              di output, spawn pipe command, apertura file/pipe per getline,
 *              flush e wait() finali. Esce da runner/mod.rs nello Step 19 Phase 4c:
 *              semantica invariata, surface minima (4 funzioni pub(super)).
 */

use crate::ast::Expr;
use crate::types::{EvalContext, InputStream, OutputStream};
use anyhow::Context;

use super::eval_expr;

/// Print/redirect handler usato da print/printf. Senza redirect scrive su
/// stdout; con `>` apre/tronca, `>>` apre in append, `|` esegue spawn pipe.
/// Riusa lo stream se la chiave (`filename`) è già nella mappa.
/// Phase 7.5: `output` è `&[u8]` byte-clean — emesso via `write_all` senza
/// passare per `Display`/`from_utf8_lossy`.
pub(super) fn handle_output(
    output: &[u8],
    redirect: &Option<(String, Expr)>,
    context: &mut EvalContext,
) -> anyhow::Result<()> {
    use std::io::Write;
    if let Some((op, file_expr)) = redirect {
        // Path file: resta String (design R3 — i path non sono dati AWK osservabili).
        let filename =
            String::from_utf8_lossy(&eval_expr(file_expr, context).as_string()).into_owned();
        use std::collections::hash_map::Entry;
        use std::fs::OpenOptions;
        let stream = match context.out_files.entry(filename.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(v) => {
                let new_stream = if op == ">>" {
                    let f = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&filename)
                        .with_context(|| format!("apertura file output '{filename}' in append"))?;
                    OutputStream::File(Box::new(f))
                } else if op == "|" {
                    use std::process::{Command, Stdio};
                    let mut child = Command::new("sh")
                        .arg("-c")
                        .arg(&filename)
                        .stdin(Stdio::piped())
                        .spawn()
                        .with_context(|| format!("spawn pipe '{filename}'"))?;
                    let stdin = child
                        .stdin
                        .take()
                        .expect("Stdio::piped garantisce stdin disponibile");
                    OutputStream::Pipe {
                        stdin: Box::new(stdin),
                        child,
                    }
                } else {
                    let f = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&filename)
                        .with_context(|| {
                            format!("apertura file output '{filename}' in scrittura")
                        })?;
                    OutputStream::File(Box::new(f))
                };
                v.insert(new_stream)
            }
        };
        stream
            .writer()
            .write_all(output)
            .with_context(|| format!("scrittura su '{filename}'"))?;
    } else {
        std::io::stdout()
            .lock()
            .write_all(output)
            .context("scrittura su stdout")?;
    }
    Ok(())
}

/// Assicura che esista uno stream di input per `filename`. Se assente, prova
/// ad aprire il file: in caso di errore lo stream resta non registrato e
/// `getline < filename` ritornerà 0 al chiamante (semantica POSIX awk).
pub(super) fn ensure_input_file(filename: &str, context: &mut EvalContext) {
    if context.in_files.contains_key(filename) {
        return;
    }
    if let Ok(file) = std::fs::File::open(filename) {
        context.in_files.insert(
            filename.to_string(),
            InputStream::File(Box::new(std::io::BufReader::new(file))),
        );
    }
}

/// Assicura che esista uno stream di input per il pipeline shell `cmd`.
/// Ritorna `false` se lo spawn fallisce — il chiamante restituirà -1.
pub(super) fn ensure_input_pipe(cmd: &str, context: &mut EvalContext) -> bool {
    if context.in_files.contains_key(cmd) {
        return true;
    }
    use std::process::{Command, Stdio};
    let child_res = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .spawn();
    match child_res {
        Ok(mut child) => {
            let stdout = child
                .stdout
                .take()
                .expect("Stdio::piped garantisce stdout disponibile");
            let reader = std::io::BufReader::new(stdout);
            context.in_files.insert(
                cmd.to_string(),
                InputStream::Pipe {
                    stdout: Box::new(reader),
                    child,
                },
            );
            true
        }
        Err(_) => false,
    }
}

/// Cleanup terminale: flush di stdout, drop+wait() su ogni pipe child rimasto
/// aperto in `out_files` / `in_files`. Chiamato a fine `run()`.
pub(super) fn flush_and_close_all(context: &mut EvalContext) {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let streams: Vec<OutputStream> = context.out_files.drain().map(|(_, v)| v).collect();
    for stream in streams {
        if let OutputStream::Pipe { stdin, mut child } = stream {
            drop(stdin);
            let _ = child.wait();
        }
    }
    let in_streams: Vec<InputStream> = context.in_files.drain().map(|(_, v)| v).collect();
    for stream in in_streams {
        if let InputStream::Pipe { stdout, mut child } = stream {
            drop(stdout);
            let _ = child.wait();
        }
    }
}
