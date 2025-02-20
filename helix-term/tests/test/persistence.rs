use super::*;
use helix_term::{config::Config, keymap};
use helix_view::editor;
use std::{fs::File, io::Read, path::Path};
use tempfile::{tempdir, NamedTempFile};

fn config_with_persistence(config_tmp_dir: &Path) -> Config {
    let mut editor_config = editor::Config::default();
    editor_config.persistence.all.enabled = true;
    editor_config.persistence.all.max_entries = 3;
    editor_config.persistence.all.scope =
        editor::PersistenceScope::Dir(config_tmp_dir.to_str().unwrap().to_string());

    Config {
        theme: None,
        keys: keymap::default(),
        editor: editor_config,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_persistence() -> anyhow::Result<()> {
    let config_tmp_dir = tempdir()?;
    let mut file = NamedTempFile::new()?;

    // Session 1:
    // open a new file,
    // add a newline, then a,
    // write-quit
    test_key_sequence(
        &mut helpers::AppBuilder::new()
            .with_config(config_with_persistence(&config_tmp_dir.path()))
            .with_file(file.path(), None)
            .build()?,
        Some("oa<esc>:wq<ret>"),
        None,
        true,
    )
    .await?;

    // Sanity check contents of file after first session
    helpers::assert_file_has_content(&mut file, &LineFeedHandling::Native.apply("\na\n"))?;

    // Session 2:
    // open same file,
    // add newline, then b,
    // copy the line ("b\n")
    // search for "a"
    // go back down to b
    // use last command (write-quit)
    test_key_sequence(
        &mut helpers::AppBuilder::new()
            .with_config(config_with_persistence(&config_tmp_dir.path()))
            .with_file(file.path(), None)
            .build()?,
        Some("ob<esc>xy/a<ret>j:<up><ret>"),
        None,
        true,
    )
    .await?;

    // This verifies both that the file position was persisted (since the b is inserted after the
    // a), and the command history (":<up>" resolves to the ":wq" from session 1)
    helpers::assert_file_has_content(&mut file, &LineFeedHandling::Native.apply("\na\nb\n"))?;

    // Session 3:
    // open same file,
    // paste
    // use last search ("/a")
    // append a
    // search for "1", "2", and "3" in sequence.
    // use last command (write-quit)
    test_key_sequence(
        &mut helpers::AppBuilder::new()
            .with_config(config_with_persistence(&config_tmp_dir.path()))
            .with_file(file.path(), None)
            .build()?,
        Some("p/<up><ret>aa<esc>/1<ret>/2<ret>/3<ret>:<up><ret>"),
        None,
        true,
    )
    .await?;

    // This verifies search history was persisted ("/<up>" resolves to "/a" from session 2), and
    // the clipboard was persisted (paste pastes the "b\n" copied in session 2)
    helpers::assert_file_has_content(&mut file, &LineFeedHandling::Native.apply("\naa\nb\nb\n"))?;

    // Session 4:
    // open same file
    // use last command (write-quit)
    test_key_sequence(
        &mut helpers::AppBuilder::new()
            .with_config(config_with_persistence(&config_tmp_dir.path()))
            .with_file(file.path(), None)
            .build()?,
        Some(":<up><ret>"),
        None,
        true,
    )
    .await?;

    // NOTE: This time we check the search history file, instead of the edited file
    let search_histfile_path = config_tmp_dir.path().join("search_history");
    let mut search_histfile = File::open(search_histfile_path)?;
    let mut search_histfile_contents = String::new();
    search_histfile.read_to_string(&mut search_histfile_contents)?;
    // This verifies that trimming the persistent state files is working correctly, because
    // session 3 sent more searches (4: "/a", "/1", "/2", "/3") than the trim limit (3), so when
    // session 4 starts, it should perform a trim, removing the oldest entry ("/a") while leaving
    // the other 3 intact.
    // The weird looking format of the string is because persistence data is encoded using bincode.
    assert_eq!(
        search_histfile_contents,
        "\u{1}\0\0\0\0\0\0\01\u{1}\0\0\0\0\0\0\02\u{1}\0\0\0\0\0\0\03"
    );

    Ok(())
}
