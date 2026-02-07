// Build script for creating bootable disk image with bootloader 0.11
//
// This script uses bootloader 0.11's builder API to create a BIOS-bootable
// disk image for x86-64. The bootloader crate is only used at build time,
// not in the final kernel binary.

use std::env;
use std::path::PathBuf;

fn main() {
    // Only run bootloader creation for x86-64 AND when bootloader feature is enabled
    let target = env::var("TARGET").unwrap();
    if !target.starts_with("x86_64") {
        // Skip for ARM64 builds - no bootloader needed
        return;
    }

    // Check if bootloader feature is enabled
    #[cfg(not(feature = "bootloader-build"))]
    {
        return;
    }

    // Get the path to the kernel binary
    // CARGO_BIN_FILE_<name> is set by cargo when building binary targets
    let kernel_path_env = env::var("CARGO_BIN_FILE_JERICHO_OS_jericho_os").ok();

    let kernel_path = if let Some(path) = kernel_path_env {
        PathBuf::from(path)
    } else {
        // Fallback: try to find the kernel in the target directory
        let target_dir = env::var("OUT_DIR").unwrap();
        let mut kernel_path = PathBuf::from(&target_dir);

        // Navigate up from OUT_DIR to find the kernel binary
        // OUT_DIR is typically: target/x86_64-jericho/debug/build/jericho_os-<hash>/out
        // We need: target/x86_64-jericho/debug/jericho_os
        for _ in 0..3 {
            kernel_path.pop();
        }
        kernel_path.push("jericho_os");

        if !kernel_path.exists() {
            // Try release build
            let mut release_path = kernel_path.clone();
            release_path.pop();
            release_path.pop();
            release_path.push("release");
            release_path.push("jericho_os");

            if release_path.exists() {
                release_path
            } else {
                println!("cargo:warning=Could not find kernel binary, skipping bootimage creation");
                return;
            }
        } else {
            kernel_path
        }
    };

    println!("cargo:warning=Building bootable disk image for: {}", kernel_path.display());

    // Get output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bios_image_path = out_dir.join("boot-bios.img");
    let uefi_image_path = out_dir.join("boot-uefi.img");

    // Create bootable disk image using bootloader 0.11 builder API
    #[cfg(feature = "bootloader-build")]
    {
        let builder = bootloader::DiskImageBuilder::new(kernel_path.clone());

        // Note: bootloader 0.11 doesn't have set_ramdisk or set_kernel_args methods
        // Configuration is done via bootloader_api's entry_point! macro in src/main.rs

        // Try UEFI boot first (recommended for bootloader 0.11)
        match builder.create_uefi_image(&uefi_image_path) {
            Ok(()) => {
                println!("cargo:warning=UEFI bootable disk image created: {}", uefi_image_path.display());
                println!("cargo:warning=Image size: {} bytes", std::fs::metadata(&uefi_image_path).unwrap().len());
                println!("cargo:warning=Use OVMF firmware to boot UEFI image");

                // Tell cargo to re-run this build script if the kernel changes
                println!("cargo:rerun-if-changed={}", kernel_path.display());
            }
            Err(e) => {
                println!("cargo:warning=Failed to create UEFI boot image: {}", e);
            }
        }

        // Also try BIOS boot as fallback
        match builder.create_bios_image(&bios_image_path) {
            Ok(()) => {
                println!("cargo:warning=BIOS bootable disk image created: {}", bios_image_path.display());
                println!("cargo:warning=Image size: {} bytes", std::fs::metadata(&bios_image_path).unwrap().len());

                // Tell cargo to re-run this build script if the kernel changes
                println!("cargo:rerun-if-changed={}", kernel_path.display());
            }
            Err(e) => {
                println!("cargo:warning=Failed to create BIOS boot image: {}", e);
                println!("cargo:warning=This is non-fatal - kernel binary still usable with manual bootloader");
            }
        }

        // Also tell cargo to re-run if this build script changes
        println!("cargo:rerun-if-changed=build.rs");
    }

    #[cfg(not(feature = "bootloader-build"))]
    {
        println!("cargo:warning=Bootloader feature not enabled, skipping image creation");
    }
}
