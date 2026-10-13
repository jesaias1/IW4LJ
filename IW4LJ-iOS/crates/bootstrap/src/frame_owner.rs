pub fn prefer_performance_cores() -> Option<PerformanceCorePlan> {
    let allowed = current_affinity()?;
    // An Intel hybrid CPU names its performance cores; a phone does not, so
    // there they are the ones the kernel rates as having the most capacity.
    let performance = match std::fs::read_to_string("/sys/devices/cpu_core/cpus") {
        Ok(list) => list,
        Err(_) if cfg!(target_os = "android") && !pinning_disabled() => {
            big_cores(&allowed)?
        }
        Err(_) => return None,
    };
    let plan = performance_core_plan(&performance, &allowed)?;
    set_affinity(&plan.chosen)?;
    Some(plan)
}

/// Efficiency cores by their ARM part number, from `/proc/cpuinfo`, which an
/// app may read where the kernel's capacity and clock files are closed to it
/// (measured: on a Galaxy S23 the pinning found nothing to pin without this).
/// Cortex-A32/A34/A35/A53/A55/A510/A520.
const LITTLE_PARTS: [u32; 7] = [0xd01, 0xd02, 0xd04, 0xd03, 0xd05, 0xd46, 0xd80];

fn big_cores_by_part(allowed: &[usize]) -> Option<String> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    let mut parts: Vec<(usize, u32)> = Vec::new();
    let mut cpu = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "processor" => cpu = value.trim().parse::<usize>().ok(),
            "CPU part" => {
                let part = u32::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok()?;
                parts.push((cpu?, part));
            }
            _ => {}
        }
    }
    let big: Vec<String> = allowed
        .iter()
        .filter(|cpu| {
            parts
                .iter()
                .find(|(number, _)| number == *cpu)
                .is_some_and(|(_, part)| !LITTLE_PARTS.contains(part))
        })
        .map(usize::to_string)
        .collect();
    (big.len() >= 3 && big.len() < allowed.len()).then(|| big.join(","))
}

/// `IW4L_PIN_CORES=0` leaves thread placement to the system.
fn pinning_disabled() -> bool {
    std::env::var("IW4L_PIN_CORES").is_ok_and(|value| value.trim() == "0")
}

/// The cores of a phone that are not its efficiency cores, as a cpu list.
///
/// A phone's scheduler keeps bursty threads on the small cores while their
/// measured load is low, and a frame is made of exactly that: dozens of short
/// tasks on a few threads that each wait for the others. One task landing on an
/// efficiency core (a quarter of the speed, at a low clock) holds up the whole
/// frame, which is how a game that uses half of every core still drops to a
/// few frames a second. Kept on the big cores, a frame's tasks finish together.
///
/// Cores are told apart by the capacity the kernel publishes, else by their top
/// clock. A phone with fewer than three big cores is left alone.
fn big_cores(allowed: &[usize]) -> Option<String> {
    if let Some(list) = big_cores_by_part(allowed) {
        return Some(list);
    }
    let read = |cpu: usize, file: &str| -> Option<u64> {
        std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{cpu}/{file}"))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    let (rated, floor_percent): (Vec<(usize, u64)>, u64) = {
        let by_capacity: Vec<_> = allowed
            .iter()
            .filter_map(|&cpu| Some((cpu, read(cpu, "cpu_capacity")?)))
            .collect();
        if by_capacity.len() == allowed.len() {
            (by_capacity, 50)
        } else {
            let by_clock = allowed
                .iter()
                .filter_map(|&cpu| Some((cpu, read(cpu, "cpufreq/cpuinfo_max_freq")?)))
                .collect::<Vec<_>>();
            if by_clock.len() != allowed.len() {
                return None;
            }
            (by_clock, 75)
        }
    };
    let top = rated.iter().map(|(_, rating)| *rating).max()?;
    let big: Vec<String> = rated
        .iter()
        .filter(|(_, rating)| rating * 100 >= top * floor_percent)
        .map(|(cpu, _)| cpu.to_string())
        .collect();
    (big.len() >= 3).then(|| big.join(","))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerformanceCorePlan {
    pub performance: Vec<usize>,

    pub allowed: Vec<usize>,

    pub chosen: Vec<usize>,
}

fn performance_core_plan(cpu_core_list: &str, allowed: &[usize]) -> Option<PerformanceCorePlan> {
    let performance = parse_cpu_list(cpu_core_list);
    let chosen: Vec<usize> = allowed
        .iter()
        .copied()
        .filter(|cpu| performance.contains(cpu))
        .collect();
    (!chosen.is_empty() && chosen.len() < allowed.len()).then(|| PerformanceCorePlan {
        performance,
        allowed: allowed.to_vec(),
        chosen,
    })
}

fn parse_cpu_list(list: &str) -> Vec<usize> {
    let mut cpus = Vec::new();
    for part in list.trim().split(',').filter(|part| !part.is_empty()) {
        let mut ends = part.split('-').map(str::parse::<usize>);
        match (ends.next(), ends.next(), ends.next()) {
            (Some(Ok(only)), None, None) => cpus.push(only),
            (Some(Ok(first)), Some(Ok(last)), None) if first <= last => cpus.extend(first..=last),
            _ => return Vec::new(),
        }
    }
    cpus.sort_unstable();
    cpus.dedup();
    cpus
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn current_affinity() -> Option<Vec<usize>> {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        (libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &raw mut set) == 0).then(|| {
            (0..libc::CPU_SETSIZE as usize)
                .filter(|cpu| libc::CPU_ISSET(*cpu, &set))
                .collect()
        })
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set_affinity(cpus: &[usize]) -> Option<()> {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        for cpu in cpus {
            if *cpu >= libc::CPU_SETSIZE as usize {
                return None;
            }
            libc::CPU_SET(*cpu, &mut set);
        }
        (libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &raw const set) == 0)
            .then_some(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn current_affinity() -> Option<Vec<usize>> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn set_affinity(_cpus: &[usize]) -> Option<()> {
    None
}
