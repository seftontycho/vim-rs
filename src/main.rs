use vim_rs::editor::Editor;

fn main() -> anyhow::Result<()> {
    let mut editor = Editor::new()?;

    editor.run()?;

    Ok(())
}
