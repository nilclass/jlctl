
pub fn run() -> anyhow::Result<()> {
    let webview = web_view::builder()
        .title("Jumperlab")
        .content(web_view::Content::Url("http://localhost:3000"))
        .user_data(27)
        .invoke_handler(|_, _| {
            Ok(())
        })
        .build()?;

    webview.run()?;

    Ok(())
}
