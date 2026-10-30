//! Windows: puts the name, the publisher, the version, the icon and a manifest into langflip.exe.
//! Defender treats an exe with no file details as more suspicious.

fn main() {
    println!("cargo:rerun-if-env-changed=LANGFLIP_VERSION");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // The release workflow sets a date version such as 2026.10.30.1. Local builds keep the Cargo one.
    let version = std::env::var("LANGFLIP_VERSION")
        .unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").unwrap());

    let mut res = winresource::WindowsResource::new();
    res.set("ProductName", "Langflip")
        .set(
            "FileDescription",
            "Langflip, fixes words typed in the wrong keyboard layout",
        )
        .set("CompanyName", "Workbench Soft")
        .set(
            "LegalCopyright",
            "Copyright (c) Workbench Soft, MIT License. Model from wordfreq data, CC BY-SA 4.0",
        )
        .set("OriginalFilename", "langflip.exe")
        .set("InternalName", "langflip")
        .set("FileVersion", &version)
        .set("ProductVersion", &version)
        .set_version_info(winresource::VersionInfo::FILEVERSION, packed(&version))
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed(&version))
        .set_icon("assets/icon.ico")
        .set_manifest(MANIFEST);
    res.compile().expect("compile the Windows resources");
}

// The binary version field holds 4 numbers of 16 bits each.
fn packed(version: &str) -> u64 {
    let mut parts = version.split('.').map(|p| p.parse::<u16>().unwrap_or(0));
    (0..4).fold(0, |acc, _| {
        (acc << 16) | u64::from(parts.next().unwrap_or(0))
    })
}

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="WorkbenchSoft.Langflip" version="1.0.0.0"/>
  <description>Langflip</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
"#;
