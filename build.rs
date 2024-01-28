fn main() -> anyhow::Result<()> {
    #[cfg(feature = "jumperlab")]
    {
        use std::process::Command;
        use std::env;
        use std::fs::copy;

        println!("cargo:rerun-if-changed=jumperlab/src");
        Command::new("jumperlab/scripts/build.sh").output()?;
        let out_dir = env::var("OUT_DIR").unwrap();
        println!("{}", std::str::from_utf8(&Command::new("ls").args(&[out_dir.clone()]).output()?.stdout).unwrap());
        copy("jumperlab/build/jumperlab.zip", &format!("{}/jumperlab.zip", out_dir))?;
    }

    shadow_rs::new()?;

    Ok(())
}
