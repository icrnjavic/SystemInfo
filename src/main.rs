use colored::Colorize;
use std::process::Command;
use std::str;

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::fs::File;
    use std::io::{self, BufRead};

    pub fn get_cpu_model() -> Option<(String, usize, usize)> {
        let output = Command::new("cat")
            .arg("/proc/cpuinfo")
            .output()
            .expect("Failed to execute command");

        let content = str::from_utf8(&output.stdout).unwrap();
        let mut model_name = None;
        let mut cores = None;
        let mut threads = 0;

        for line in content.lines() {
            if line.starts_with("model name") {
                if model_name.is_none() {
                    model_name = Some(line.split(':').nth(1)?.trim().to_string());
                }
            }
            if line.starts_with("cpu cores") {
                if cores.is_none() {
                    cores = Some(line.split(':').nth(1)?.trim().parse().unwrap_or(1));
                }
            }
            if line.starts_with("processor") {
                threads += 1;
            }
        }
        model_name.map(|name| (name, cores.unwrap_or(1), threads))
    }

    pub fn get_gpu_model() -> Option<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -i vga")
            .output()
            .expect("Failed to execute command");

        let content = str::from_utf8(&output.stdout).unwrap();
        let re = regex::Regex::new(r"VGA compatible controller: (.+)").unwrap();
        for line in content.lines() {
            if let Some(cap) = re.captures(line) {
                return Some(cap[1].to_string());
            }
        }
        None
    }

    pub fn get_ram_info() -> Option<(f64, f64, f64)> {
        let total_mem = Command::new("grep")
            .arg("MemTotal")
            .arg("/proc/meminfo")
            .output()
            .expect("Failed to execute command");
        let free_mem = Command::new("grep")
            .arg("MemAvailable")
            .arg("/proc/meminfo")
            .output()
            .expect("Failed to execute command");

        let total_mem_content = str::from_utf8(&total_mem.stdout).unwrap();
        let free_mem_content = str::from_utf8(&free_mem.stdout).unwrap();

        let total = total_mem_content
            .split_whitespace()
            .nth(1)
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0) as f64;
        let available = free_mem_content
            .split_whitespace()
            .nth(1)
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0) as f64;
        let used = total - available;
        let usage_percent = (used / total) * 100.0;

        // Convert KB to GB
        Some((used / 1_048_576.0, total / 1_048_576.0, usage_percent))
    }

    pub fn get_os_version() -> Option<String> {
        let path = "/etc/os-release";
        if let Ok(file) = File::open(path) {
            for line in io::BufReader::new(file).lines() {
                if let Ok(line) = line {
                    if line.starts_with("PRETTY_NAME=") {
                        let os_version = line.split('=').nth(1)?.trim_matches('"').to_string();
                        return Some(os_version);
                    }
                }
            }
        }
        None
    }

    
    pub fn get_hostname() -> Option<String> {
        let output = Command::new("hostname")
            .output()
            .expect("Failed to execute command");

        let hostname = str::from_utf8(&output.stdout).ok()?.trim().to_string();
        Some(hostname)
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    pub fn get_cpu_model() -> Option<(String, usize, usize)> {
        let output = Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()
            .expect("Failed to execute command");

        let cpu_model = str::from_utf8(&output.stdout).ok()?.trim().to_string();

        let cores_output = Command::new("sysctl")
            .arg("-n")
            .arg("hw.physicalcpu")
            .output()
            .expect("Failed to execute command");
        let cores = str::from_utf8(&cores_output.stdout).ok()?.trim().parse().unwrap_or(1);

        let threads_output = Command::new("sysctl")
            .arg("-n")
            .arg("hw.logicalcpu")
            .output()
            .expect("Failed to execute command");
        let threads = str::from_utf8(&threads_output.stdout).ok()?.trim().parse().unwrap_or(1);

        Some((cpu_model, cores, threads))
    }

    pub fn get_gpu_model() -> Option<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg("system_profiler SPDisplaysDataType | grep 'Chipset Model'")
            .output()
            .expect("Failed to execute command");

        let content = str::from_utf8(&output.stdout).unwrap();
        let re = regex::Regex::new(r"Chipset Model: (.+)").unwrap();
        for line in content.lines() {
            if let Some(cap) = re.captures(line) {
                return Some(cap[1].to_string());
            }
        }
        None
    }

    pub fn get_ram_info() -> Option<(f64, f64, f64)> {
        let total_mem = Command::new("sysctl")
            .arg("hw.memsize")
            .output()
            .expect("Failed to execute command");
        let used_mem = Command::new("vm_stat")
            .output()
            .expect("Failed to execute command");

        let total_mem_content = str::from_utf8(&total_mem.stdout).unwrap();
        let used_mem_content = str::from_utf8(&used_mem.stdout).unwrap();

        let total = total_mem_content
            .split_whitespace()
            .nth(1)
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0) as f64;

        let used_re = regex::Regex::new(r"Pages active: (\d+)").unwrap();
        let free_re = regex::Regex::new(r"Pages free: (\d+)").unwrap();
        let page_size_re = regex::Regex::new(r"page size of (\d+) bytes").unwrap();

        let used_pages = used_re.captures(used_mem_content).and_then(|cap| cap.get(1)).map_or(0, |m| m.as_str().parse::<u64>().unwrap());
        let free_pages = free_re.captures(used_mem_content).and_then(|cap| cap.get(1)).map_or(0, |m| m.as_str().parse::<u64>().unwrap());
        let page_size = page_size_re.captures(used_mem_content).and_then(|cap| cap.get(1)).map_or(4096, |m| m.as_str().parse::<u64>().unwrap());

        let used = (used_pages * page_size) as f64 / 1024.0;
        let usage_percent = (used / total) * 100.0;

        // Convert KB to GB
        Some((used / 1_048_576.0, total / 1_048_576.0, usage_percent))
    }

    pub fn get_os_version() -> Option<String> {
        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .expect("Failed to execute command");

        let os_version = str::from_utf8(&output.stdout).ok()?.trim().to_string();
        Some(os_version)
    }

    pub fn get_system_name() -> Option<String> {
        let output = Command::new("scutil")
            .arg("--get")
            .arg("ComputerName")
            .output()
            .expect("Failed to execute command");

        let system_name = str::from_utf8(&output.stdout).ok()?.trim().to_string();
        Some(system_name)
    }

    pub fn get_hostname() -> Option<String> {
        let output = Command::new("hostname")
            .output()
            .expect("Failed to execute command");

        let hostname = str::from_utf8(&output.stdout).ok()?.trim().to_string();
        Some(hostname)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    pub fn get_cpu_model() -> Option<(String, usize, usize)> {
        let model_output = Command::new("wmic")
            .arg("cpu")
            .arg("get")
            .arg("Name")
            .output()
            .expect("Failed to execute command");
        let model_content = str::from_utf8(&model_output.stdout).unwrap();
        let model_lines: Vec<&str> = model_content.lines().collect();
        let model_name = if model_lines.len() > 1 {
            model_lines[1].trim().to_string()
        } else {
            "Unknown".to_string()
        };

        let cores_output = Command::new("wmic")
            .arg("cpu")
            .arg("get")
            .arg("NumberOfCores")
            .output()
            .expect("Failed to execute command");
        let cores_content = str::from_utf8(&cores_output.stdout).unwrap();
        let cores_lines: Vec<&str> = cores_content.lines().collect();
        let cores = if cores_lines.len() > 1 {
            cores_lines[1].trim().parse().unwrap_or(1)
        } else {
            1
        };

        let threads_output = Command::new("wmic")
            .arg("cpu")
            .arg("get")
            .arg("NumberOfLogicalProcessors")
            .output()
            .expect("Failed to execute command");
        let threads_content = str::from_utf8(&threads_output.stdout).unwrap();
        let threads_lines: Vec<&str> = threads_content.lines().collect();
        let threads = if threads_lines.len() > 1 {
            threads_lines[1].trim().parse().unwrap_or(1)
        } else {
            1
        };

        Some((model_name, cores, threads))
    }

    pub fn get_gpu_model() -> Option<String> {
        let output = Command::new("wmic")
            .arg("path")
            .arg("win32_videocontroller")
            .arg("get")
            .arg("name")
            .output()
            .expect("Failed to execute command");

        let content = str::from_utf8(&output.stdout).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 1 {
            return Some(lines[1].trim().to_string());
        }
        None
    }

    pub fn get_ram_info() -> Option<(f64, f64, f64)> {
        let total_mem = Command::new("wmic")
            .arg("ComputerSystem")
            .arg("get")
            .arg("TotalPhysicalMemory")
            .output()
            .expect("Failed to execute command");
        let free_mem = Command::new("wmic")
            .arg("OS")
            .arg("get")
            .arg("FreePhysicalMemory")
            .output()
            .expect("Failed to execute command");

        let total_mem_content = str::from_utf8(&total_mem.stdout).unwrap();
        let free_mem_content = str::from_utf8(&free_mem.stdout).unwrap();

        let total: u64 = total_mem_content
            .lines()
            .nth(1)
            .unwrap_or("0")
            .trim()
            .parse()
            .unwrap_or(0) / 1024; // Convert to KB
        let free: u64 = free_mem_content
            .lines()
            .nth(1)
            .unwrap_or("0")
            .trim()
            .parse()
            .unwrap_or(0);
        let used = total - free;
        let usage_percent = (used as f64 / total as f64) * 100.0;

        // Convert KB to GB
        Some((used as f64 / 1_048_576.0, total as f64 / 1_048_576.0, usage_percent))
    }

    pub fn get_os_version() -> Option<String> {
        let output = Command::new("wmic")
            .arg("os")
            .arg("get")
            .arg("Caption")
            .output()
            .expect("Failed to execute command");

        let content = str::from_utf8(&output.stdout).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 1 {
            return Some(lines[1].trim().to_string());
        }
        None
    }

    pub fn get_system_name() -> Option<String> {
        let output = Command::new("wmic")
            .arg("computersystem")
            .arg("get")
            .arg("Name")
            .output()
            .expect("Failed to execute command");

        let content = str::from_utf8(&output.stdout).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 1 {
            return Some(lines[1].trim().to_string());
        }
        None
    }

    pub fn get_hostname() -> Option<String> {
        let output = Command::new("hostname")
            .output()
            .expect("Failed to execute command");

        let hostname = str::from_utf8(&output.stdout).ok()?.trim().to_string();
        Some(hostname)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    pub fn get_cpu_model() -> Option<(String, usize, usize)> {
        None
    }

    pub fn get_gpu_model() -> Option<String> {
        None
    }

    pub fn get_ram_info() -> Option<(f64, f64, f64)> {
        None
    }

    pub fn get_os_version() -> Option<String> {
        None
    }

    pub fn get_system_name() -> Option<String> {
        None
    }

    pub fn get_hostname() -> Option<String> {
        None
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    //println!("{}", "Linux system detected.".green());

    #[cfg(target_os = "macos")]
    //println!("{}", "macOS system detected.".green());

    #[cfg(target_os = "windows")]
    //println!("{}", "Windows system detected.".green());

    
    if let Some(hostname) = platform::get_hostname() {
        println!("{}: {}", "Hostname".red().bold(), hostname.bold());
    } else {
        println!("{}", "Could not retrieve hostname.".red().bold());
    }

    if let Some((cpu_model, cores, threads)) = platform::get_cpu_model() {
        println!(
            "{}: {} ({} cores/{} threads)",
            "CPU Model".red().bold(),
            cpu_model.bold(),
            cores.to_string().bold(),
            threads.to_string().bold()
        );
    } else {
        println!("{}", "Could not retrieve CPU model.".red().bold());
    }

    if let Some(gpu_model) = platform::get_gpu_model() {
        println!("{}: {}", "GPU Model".red().bold(), gpu_model.bold());
    } else {
        println!("{}", "Could not retrieve GPU model.".red().bold());
    }

    if let Some((used_ram, total_ram, usage_percent)) = platform::get_ram_info() {
        println!(
            "{}: {:.4} GB / {:.4} GB ({}%)",
            "RAM".red().bold(),
            format!("{:.4}", used_ram).bold(),
            format!("{:.4}", total_ram).bold(),
            format!("{:.2}", usage_percent).bold()
        );
    } else {
        println!("{}", "Could not retrieve RAM info.".red().bold());
    }

    if let Some(os_version) = platform::get_os_version() {
        println!("{}: {}", "OS Version".red().bold(), os_version.bold());
    } else {
        println!("{}", "Could not retrieve OS version.".red().bold());
    }

}
