use std::io::{self, Write};

pub fn write_stdout_text(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    write_with_broken_pipe_tolerance(&mut stdout, text.as_bytes())?;
    flush_with_broken_pipe_tolerance(&mut stdout)
}

pub fn write_stdout_line(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    write_with_broken_pipe_tolerance(&mut stdout, text.as_bytes())?;
    write_with_broken_pipe_tolerance(&mut stdout, b"\n")?;
    flush_with_broken_pipe_tolerance(&mut stdout)
}

fn write_with_broken_pipe_tolerance(writer: &mut dyn Write, bytes: &[u8]) -> io::Result<()> {
    match writer.write_all(bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
    }
}

fn flush_with_broken_pipe_tolerance(writer: &mut dyn Write) -> io::Result<()> {
    match writer.flush() {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
    }
}
