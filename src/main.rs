use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "npm_package_check",
    about = "检查 pnpm-lock.yaml 文件中是否包含指定的包和版本"
)]
struct Args {
    #[arg(help = "要查找的包名（例如：antd 或 @ant-design/icons）")]
    package: Option<String>,

    #[arg(help = "版本号（可选，不指定则匹配任意版本）")]
    version: Option<String>,

    #[arg(
        short,
        long,
        default_value = "pnpm-lock.yaml",
        help = "pnpm-lock.yaml 文件路径"
    )]
    file: String,

    #[arg(short, long, help = "显示详细信息")]
    verbose: bool,
    
    #[arg(short, long, help = "批量检查模式：指定包列表文件路径")]
    batch: Option<String>,
    
    #[arg(long, help = "输出报告文件路径（批量模式）")]
    output: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PnpmLock {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: String,
    
    #[serde(default)]
    importers: HashMap<String, Importer>,
    
    #[serde(default)]
    packages: HashMap<String, PackageInfo>,
    
    #[serde(default)]
    snapshots: HashMap<String, SnapshotInfo>,
}

#[derive(Debug, Deserialize)]
struct Importer {
    #[serde(default)]
    dependencies: HashMap<String, DependencyInfo>,
    
    #[serde(default)]
    #[serde(rename = "devDependencies")]
    dev_dependencies: HashMap<String, DependencyInfo>,
    
    #[serde(default)]
    #[serde(rename = "optionalDependencies")]
    optional_dependencies: HashMap<String, DependencyInfo>,
}

#[derive(Debug, Deserialize)]
struct DependencyInfo {
    specifier: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
    resolution: Resolution,
    
    #[serde(default)]
    #[serde(rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,
    
    #[serde(default)]
    dependencies: HashMap<String, String>,
    
    #[serde(default)]
    #[serde(rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Resolution {
    integrity: String,
    
    #[serde(default)]
    tarball: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SnapshotInfo {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    
    #[serde(default)]
    #[serde(rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
    
    #[serde(default)]
    #[serde(rename = "optionalDependencies")]
    optional_dependencies: HashMap<String, String>,
}

#[derive(Debug)]
struct PackageFound {
    location: String,
    specifier: String,
    version: String,
    dependency_type: String,
}

#[derive(Debug, Clone)]
struct BatchPackage {
    name: String,
    versions: Vec<String>,
    status: Option<String>,
    detection_date: Option<String>,
}

#[derive(Debug)]
struct BatchResult {
    package: BatchPackage,
    found_versions: Vec<PackageFound>,
    status: CheckStatus,
}

#[derive(Debug, PartialEq)]
enum CheckStatus {
    Found,
    VersionMismatch,
    NotFound,
    PartialMatch,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let file_path = Path::new(&args.file);
    if !file_path.exists() {
        eprintln!("错误：文件 '{}' 不存在", args.file);
        std::process::exit(1);
    }
    
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("无法读取文件 '{}'", args.file))?;
    
    let lock_data: PnpmLock = serde_yaml::from_str(&content)
        .with_context(|| "解析 pnpm-lock.yaml 文件失败")?;
    
    if let Some(ref batch_file) = args.batch {
        // 批量检查模式
        run_batch_check(&args, &lock_data, batch_file)?;
    } else {
        // 单包检查模式
        if let Some(ref package_name) = args.package {
            run_single_check(&args, &lock_data, package_name)?;
        } else {
            eprintln!("错误：必须指定包名或使用批量模式(-b/--batch)");
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn run_single_check(args: &Args, lock_data: &PnpmLock, package_name: &str) -> Result<()> {
    if args.verbose {
        println!("Lockfile 版本: {}", lock_data.lockfile_version);
        println!("正在查找包: {}", package_name);
        if let Some(ref version) = args.version {
            println!("指定版本: {}", version);
        }
        println!("---");
    }
    
    let found_packages = find_package_in_lock(lock_data, package_name);
    
    // 输出结果
    if found_packages.is_empty() {
        println!("❌ 未找到包: {}", package_name);
        std::process::exit(1);
    } else {
        // 如果指定了版本，过滤结果
        if let Some(ref target_version) = args.version {
            let matched: Vec<_> = found_packages
                .iter()
                .filter(|p| version_matches(&p.version, target_version))
                .collect();
            
            if matched.is_empty() {
                println!("❌ 找到包 '{}' 但版本不匹配", package_name);
                println!("   期望版本: {}", target_version);
                println!("   实际版本:");
                for pkg in &found_packages {
                    println!("   - {} ({})", pkg.version, pkg.location);
                }
                std::process::exit(1);
            } else {
                println!("✅ 找到包: {} @ {}", package_name, target_version);
                for pkg in matched {
                    print_package_info(pkg, args.verbose);
                }
            }
        } else {
            println!("✅ 找到包: {}", package_name);
            for pkg in &found_packages {
                print_package_info(pkg, args.verbose);
            }
        }
    }
    
    Ok(())
}

fn run_batch_check(args: &Args, lock_data: &PnpmLock, batch_file: &str) -> Result<()> {
    let batch_packages = parse_batch_file(batch_file)?;
    
    if args.verbose {
        println!("Lockfile 版本: {}", lock_data.lockfile_version);
        println!("批量检查模式: {} 个包", batch_packages.len());
        println!("---");
    }
    
    let mut results = Vec::new();
    
    for package in &batch_packages {
        let found_packages = find_package_in_lock(lock_data, &package.name);
        
        let status = if found_packages.is_empty() {
            CheckStatus::NotFound
        } else if package.versions.is_empty() {
            CheckStatus::Found
        } else {
            let matched_versions: Vec<_> = found_packages
                .iter()
                .filter(|p| package.versions.iter().any(|v| version_matches(&p.version, v)))
                .collect();
            
            if matched_versions.is_empty() {
                CheckStatus::VersionMismatch
            } else if matched_versions.len() == package.versions.len() {
                CheckStatus::Found
            } else {
                CheckStatus::PartialMatch
            }
        };
        
        results.push(BatchResult {
            package: package.clone(),
            found_versions: found_packages,
            status,
        });
    }
    
    // 输出批量检查结果
    print_batch_results(&results, args.verbose);
    
    // 如果指定了输出文件，写入报告
    if let Some(output_file) = &args.output {
        write_batch_report(&results, output_file)?;
        println!("\n📊 报告已写入: {}", output_file);
    }
    
    Ok(())
}

fn find_package_in_lock(lock_data: &PnpmLock, package_name: &str) -> Vec<PackageFound> {
    let mut found_packages = Vec::new();
    
    // 在 importers 中查找
    for (importer_path, importer) in &lock_data.importers {
        let display_path = if importer_path == "." {
            "根目录".to_string()
        } else {
            importer_path.clone()
        };
        
        // 检查 dependencies
        if let Some(dep_info) = importer.dependencies.get(package_name) {
            found_packages.push(PackageFound {
                location: display_path.clone(),
                specifier: dep_info.specifier.clone(),
                version: extract_version(&dep_info.version),
                dependency_type: "dependencies".to_string(),
            });
        }
        
        // 检查 devDependencies
        if let Some(dep_info) = importer.dev_dependencies.get(package_name) {
            found_packages.push(PackageFound {
                location: display_path.clone(),
                specifier: dep_info.specifier.clone(),
                version: extract_version(&dep_info.version),
                dependency_type: "devDependencies".to_string(),
            });
        }
        
        // 检查 optionalDependencies
        if let Some(dep_info) = importer.optional_dependencies.get(package_name) {
            found_packages.push(PackageFound {
                location: display_path,
                specifier: dep_info.specifier.clone(),
                version: extract_version(&dep_info.version),
                dependency_type: "optionalDependencies".to_string(),
            });
        }
    }
    
    // 在 packages 中查找
    let package_patterns = vec![
        format!("{}@", package_name),
        format!("/{}@", package_name),
    ];
    
    for (package_key, _package_info) in &lock_data.packages {
        for pattern in &package_patterns {
            if package_key.contains(pattern) {
                let version = extract_version_from_key(package_key, package_name);
                if !found_packages.iter().any(|p| p.version == version) {
                    found_packages.push(PackageFound {
                        location: "packages节点".to_string(),
                        specifier: "".to_string(),
                        version: version.clone(),
                        dependency_type: "packages".to_string(),
                    });
                }
            }
        }
    }
    
    // 在 snapshots 中查找
    for (snapshot_key, snapshot_info) in &lock_data.snapshots {
        let key_without_version = extract_package_name_from_snapshot_key(snapshot_key);
        
        // 检查 snapshot 的 dependencies
        if let Some(dep_version) = snapshot_info.dependencies.get(package_name) {
            let version = extract_version(dep_version);
            if !found_packages.iter().any(|p| p.version == version && p.location == "snapshots节点") {
                found_packages.push(PackageFound {
                    location: "snapshots节点".to_string(),
                    specifier: "".to_string(),
                    version: version.clone(),
                    dependency_type: format!("snapshots[{}].dependencies", snapshot_key),
                });
            }
        }
        
        // 检查包名是否匹配 snapshot key 本身
        if key_without_version == package_name || key_without_version.ends_with(&format!("/{}", package_name)) {
            let version = extract_version_from_snapshot_key(snapshot_key);
            if !version.is_empty() && !found_packages.iter().any(|p| p.version == version && p.location == "snapshots节点") {
                found_packages.push(PackageFound {
                    location: "snapshots节点".to_string(),
                    specifier: "".to_string(),
                    version,
                    dependency_type: "snapshots".to_string(),
                });
            }
        }
    }
    
    found_packages
}

fn parse_batch_file(file_path: &str) -> Result<Vec<BatchPackage>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("无法读取批量文件 '{}'", file_path))?;
    
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    
    // 检测文件格式
    let header = lines[0];
    if header.contains("Package Name\tVersion(s)") {
        // version1.txt 格式
        parse_version1_format(&lines[1..])
    } else if header.contains("Package Name\tCompromised Version(s)\tDetection Date\tStatus") {
        // version2.txt 格式  
        parse_version2_format(&lines[1..])
    } else {
        Err(anyhow::anyhow!("无法识别的文件格式：{}", header))
    }
}

fn parse_version1_format(lines: &[&str]) -> Result<Vec<BatchPackage>> {
    let mut packages = Vec::new();
    
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        
        let package_name = parts[1].trim().to_string();
        let versions_str = parts[2].trim();
        let versions: Vec<String> = if versions_str.is_empty() {
            Vec::new()
        } else {
            versions_str.split(", ").map(|s| s.trim().to_string()).collect()
        };
        
        packages.push(BatchPackage {
            name: package_name,
            versions,
            status: None,
            detection_date: None,
        });
    }
    
    Ok(packages)
}

fn parse_version2_format(lines: &[&str]) -> Result<Vec<BatchPackage>> {
    let mut packages = Vec::new();
    
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }
        
        let package_name = parts[0].trim().to_string();
        let versions_str = parts[1].trim();
        let detection_date = Some(parts[2].trim().to_string());
        let status = Some(parts[3].trim().to_string());
        
        let versions: Vec<String> = if versions_str.is_empty() {
            Vec::new()
        } else {
            versions_str.split(", ").map(|s| s.trim().to_string()).collect()
        };
        
        packages.push(BatchPackage {
            name: package_name,
            versions,
            status,
            detection_date,
        });
    }
    
    Ok(packages)
}
fn extract_version(version_str: &str) -> String {
    // 从版本字符串中提取纯版本号
    // 例如: "4.8.3(react-dom@18.3.1)(react@18.3.1)" -> "4.8.3"
    if let Some(pos) = version_str.find('(') {
        version_str[..pos].to_string()
    } else {
        version_str.to_string()
    }
}

fn print_batch_results(results: &[BatchResult], verbose: bool) {
    let mut found_count = 0;
    let mut not_found_count = 0;
    let mut version_mismatch_count = 0;
    let mut partial_match_count = 0;
    
    println!("📊 批量检查结果:\n");
    
    for result in results {
        let status_icon = match result.status {
            CheckStatus::Found => {
                found_count += 1;
                "✅"
            }
            CheckStatus::NotFound => {
                not_found_count += 1;
                "❌"
            }
            CheckStatus::VersionMismatch => {
                version_mismatch_count += 1;
                "⚠️"
            }
            CheckStatus::PartialMatch => {
                partial_match_count += 1;
                "🟡"
            }
        };
        
        println!("{} {}", status_icon, result.package.name);
        
        if verbose || result.status != CheckStatus::Found {
            println!("   预期版本: {}", 
                if result.package.versions.is_empty() { 
                    "任意版本".to_string() 
                } else { 
                    result.package.versions.join(", ") 
                });
            
            if result.status != CheckStatus::NotFound {
                println!("   实际版本:");
                for pkg in &result.found_versions {
                    println!("   - {} @ {} ({})", pkg.location, pkg.version, pkg.dependency_type);
                }
            }
            
            if let Some(ref status) = result.package.status {
                println!("   状态: {}", status);
            }
            
            if let Some(ref date) = result.package.detection_date {
                println!("   检测日期: {}", date);
            }
            
            println!();
        }
    }
    
    println!("🎯 统计信息:");
    println!("   总数: {}", results.len());
    println!("   ✅ 找到: {}", found_count);
    println!("   🟡 部分匹配: {}", partial_match_count);
    println!("   ⚠️ 版本不匹配: {}", version_mismatch_count);
    println!("   ❌ 未找到: {}", not_found_count);
}

fn write_batch_report(results: &[BatchResult], output_file: &str) -> Result<()> {
    use std::io::Write;
    
    let mut file = std::fs::File::create(output_file)
        .with_context(|| format!("无法创建输出文件 '{}'", output_file))?;
    
    writeln!(file, "Package Name\tStatus\tExpected Versions\tFound Versions\tLocations\tOriginal Status\tDetection Date")?;
    
    for result in results {
        let status_text = match result.status {
            CheckStatus::Found => "Found",
            CheckStatus::NotFound => "Not Found",
            CheckStatus::VersionMismatch => "Version Mismatch",
            CheckStatus::PartialMatch => "Partial Match",
        };
        
        let expected_versions = if result.package.versions.is_empty() {
            "Any".to_string()
        } else {
            result.package.versions.join(", ")
        };
        
        let found_versions = if result.found_versions.is_empty() {
            "None".to_string()
        } else {
            result.found_versions.iter()
                .map(|p| p.version.clone())
                .collect::<Vec<_>>()
                .join(", ")
        };
        
        let locations = if result.found_versions.is_empty() {
            "None".to_string()
        } else {
            result.found_versions.iter()
                .map(|p| format!("{} ({})", p.location, p.dependency_type))
                .collect::<Vec<_>>()
                .join("; ")
        };
        
        let original_status = result.package.status.as_deref().unwrap_or("");
        let detection_date = result.package.detection_date.as_deref().unwrap_or("");
        
        writeln!(file, "{}\t{}\t{}\t{}\t{}\t{}\t{}", 
            result.package.name,
            status_text,
            expected_versions,
            found_versions,
            locations,
            original_status,
            detection_date
        )?;
    }
    
    Ok(())
}

fn extract_version_from_key(key: &str, package_name: &str) -> String {
    // 从 packages key 中提取版本号
    // 例如: "@ant-design/icons@4.8.3" -> "4.8.3"
    let patterns = vec![
        format!("{}@", package_name),
        format!("/{}@", package_name),
    ];
    
    for pattern in patterns {
        if let Some(pos) = key.find(&pattern) {
            let start = pos + pattern.len();
            return key[start..].split('_').next().unwrap_or("").to_string();
        }
    }
    
    String::new()
}

fn version_matches(actual: &str, expected: &str) -> bool {
    // 简单的版本匹配
    // 可以扩展支持语义化版本匹配（^, ~, >=, 等）
    actual == expected || actual.starts_with(&format!("{}.", expected))
}

fn extract_package_name_from_snapshot_key(key: &str) -> String {
    // 从 snapshot key 中提取包名
    // 例如: "@ahooksjs/use-request@2.8.15(react@18.3.1)" -> "@ahooksjs/use-request"
    if let Some(at_pos) = key.rfind('@') {
        // 找到最后一个@，它之前的是包名
        let package_part = &key[..at_pos];
        // 处理可能的括号情况
        if let Some(paren_pos) = package_part.find('(') {
            package_part[..paren_pos].to_string()
        } else {
            package_part.to_string()
        }
    } else if let Some(paren_pos) = key.find('(') {
        key[..paren_pos].to_string()
    } else {
        key.to_string()
    }
}

fn extract_version_from_snapshot_key(key: &str) -> String {
    // 从 snapshot key 中提取版本号
    // 例如: "@ahooksjs/use-request@2.8.15(react@18.3.1)" -> "2.8.15"
    if let Some(at_pos) = key.rfind('@') {
        let after_at = &key[at_pos + 1..];
        // 版本号在括号之前或到字符串结束
        if let Some(paren_pos) = after_at.find('(') {
            after_at[..paren_pos].to_string()
        } else {
            after_at.to_string()
        }
    } else {
        String::new()
    }
}

fn print_package_info(pkg: &PackageFound, verbose: bool) {
    if verbose {
        println!("   📍 位置: {}", pkg.location);
        println!("      类型: {}", pkg.dependency_type);
        if !pkg.specifier.is_empty() {
            println!("      规格: {}", pkg.specifier);
        }
        println!("      版本: {}", pkg.version);
        println!();
    } else {
        println!("   {} @ {} ({})", pkg.location, pkg.version, pkg.dependency_type);
    }
}