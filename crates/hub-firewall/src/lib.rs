//! Windows' firewall and the phone and tablet companion (`hub_api::companion::firewall`): whether
//! Windows lets the phones in, and setting that right - read and written through the firewall's
//! own API (`INetFwPolicy2`), whatever language Windows speaks.
//!
//! Setting it right needs the administrator's rights. Omsi-Hub asks for them for itself: it starts
//! its own program again with [`ARG`] and the port (`ShellExecuteEx` "runas", Windows' UAC
//! question), and that copy sets the rules ([`from_args`]) and ends before any window opens. No
//! script and no PowerShell: an unsigned program starting a hidden PowerShell with a coded command
//! and the administrator's rights is what malware does, and the virus scanners took Omsi-Hub for it
//! (card #4: Norton, Windows Security).

/// The rule's name in Windows' firewall.
pub const RULE: &str = "Omsi-Hub Phone & tablet";

/// Its description.
pub const ABOUT: &str = "Lets phones and tablets in the home network reach Omsi-Hub (Settings > General > Phone & tablet).";

/// The argument that starts Omsi-Hub as the firewall's setter, the port after it.
pub const ARG: &str = "--companion-firewall";

/// An inbound or outbound rule as the check reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rule {
    pub name: String,
    /// Its program's path, its variables (`%ProgramFiles%`) filled in; empty: every program.
    pub program: String,
    pub inbound: bool,
    pub enabled: bool,
    pub allow: bool,
    /// Its local ports as the firewall writes them: `47811`, `80,443`, `5000-6000`, `*`.
    pub ports: String,
}

/// Whether `path` is the program `exe` (Windows' paths: the case and the slashes do not count).
pub fn same_program(path: &str, exe: &str) -> bool {
    let norm = |s: &str| s.trim().replace('/', "\\").to_lowercase();
    !path.trim().is_empty() && norm(path) == norm(exe)
}

/// Whether a rule's local ports take in `port`.
pub fn takes_port(ports: &str, port: u16) -> bool {
    ports.split(',').map(str::trim).any(|p| {
        if p == "*" || p.eq_ignore_ascii_case("any") {
            return true;
        }
        match p.split_once('-') {
            Some((a, b)) => matches!((a.trim().parse::<u16>(), b.trim().parse::<u16>()), (Ok(a), Ok(b)) if (a..=b).contains(&port)),
            None => p.parse::<u16>() == Ok(port),
        }
    })
}

/// Whether rule `r` keeps the phones out: inbound, on, blocking the program `exe`.
pub fn blocks(r: &Rule, exe: &str) -> bool {
    r.inbound && r.enabled && !r.allow && same_program(&r.program, exe)
}

/// Whether the phones get in on `port`: the firewall off on every kind of network (`profiles_on`
/// 0), or no rule blocking `exe` and Omsi-Hub's own rule letting the port in.
pub fn open(profiles_on: u32, rules: &[Rule], exe: &str, port: u16) -> bool {
    if profiles_on == 0 {
        return true;
    }
    let ours = rules.iter().any(|r| r.name == RULE && r.inbound && r.enabled && r.allow && takes_port(&r.ports, port));
    !rules.iter().any(|r| blocks(r, exe)) && ours
}

/// The port Omsi-Hub was started with to set the firewall right ([`ARG`]): None when it was not,
/// `Some(None)` when the port is not one.
pub fn port_of_args(args: &[String]) -> Option<Option<u16>> {
    let k = args.iter().position(|a| a == ARG)?;
    Some(args.get(k + 1).and_then(|p| p.parse::<u16>().ok()).filter(|p| *p >= 1024))
}

/// In the copy of Omsi-Hub started to set the firewall right: the rules set for this program and
/// the exit code (0: done, 1: it failed, 2: no port given). None when this is not that copy - the
/// app goes on.
pub fn from_args() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let port = port_of_args(&args)?;
    let Some(port) = port else { return Some(2) };
    #[cfg(windows)]
    {
        let Ok(exe) = std::env::current_exe() else { return Some(1) };
        Some(if win::set_right(&exe.to_string_lossy(), port).is_ok() { 0 } else { 1 })
    }
    #[cfg(not(windows))]
    {
        let _ = port;
        Some(1)
    }
}

/// Whether the phones get in on `port` to the program `exe` (None: not Windows, or it could not
/// tell).
pub fn check(exe: &str, port: u16) -> Option<bool> {
    #[cfg(windows)]
    {
        win::check(exe, port).ok()
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, port);
        None
    }
}

/// Set it right, asking Windows for the rights (UAC) for the program `exe` itself: Err(true) when
/// the player said no, Err(false) when it failed.
pub fn fix(exe: &str, port: u16) -> Result<(), bool> {
    #[cfg(windows)]
    {
        win::elevated(exe, port)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, port);
        Err(false)
    }
}

#[cfg(windows)]
mod win {
    use super::{blocks, open, Rule, ABOUT, ARG, RULE};
    use windows::core::{Interface, BSTR, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, VARIANT_FALSE, VARIANT_TRUE};
    use windows::Win32::NetworkManagement::WindowsFirewall::*;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
    use windows::Win32::System::Ole::IEnumVARIANT;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    /// COM on this thread while it lives.
    struct Com(bool);

    impl Com {
        fn new() -> Self {
            // (S_OK and S_FALSE are each undone; a thread already in another mode is left as it is)
            Com(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok())
        }
    }

    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    fn policy() -> windows::core::Result<INetFwPolicy2> {
        unsafe { CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER) }
    }

    /// A path with its variables filled in (`%ProgramFiles%\…`).
    fn expand(s: &str) -> String {
        if !s.contains('%') {
            return s.to_string();
        }
        let src = HSTRING::from(s);
        let n = unsafe { ExpandEnvironmentStringsW(&src, None) } as usize;
        let mut buf = vec![0u16; n.max(1)];
        let got = unsafe { ExpandEnvironmentStringsW(&src, Some(&mut buf)) } as usize;
        if got == 0 || got > buf.len() {
            return s.to_string();
        }
        String::from_utf16_lossy(&buf[..got.saturating_sub(1)])
    }

    /// Every rule of the firewall's list `list`, with what the check reads of it (the list kept
    /// by the caller while the rules are used).
    fn rules(list: &INetFwRules) -> windows::core::Result<Vec<(INetFwRule, Rule)>> {
        let all = unsafe { list._NewEnum()? }.cast::<IEnumVARIANT>()?;
        let mut out = Vec::new();
        loop {
            let mut v = [VARIANT::default()];
            let mut got = 0u32;
            if unsafe { all.Next(&mut v, &mut got) }.is_err() || got == 0 {
                break;
            }
            let [v] = v;
            // (the rule borrowed from the VARIANT, never taken out of it: the firewall's enumerator
            // keeps the reference it hands out and releases it itself when it goes - taken and
            // released here as well, every rule was released once too often, and Omsi-Hub 1.2.0
            // crashed as it started with the phone & tablet on)
            let inner = unsafe { &v.Anonymous.Anonymous };
            if inner.vt != VT_DISPATCH {
                continue;
            }
            let Some(d): Option<IDispatch> = (unsafe { (*inner.Anonymous.pdispVal).clone() }) else { continue };
            let Ok(r) = d.cast::<INetFwRule>() else { continue };
            let read = unsafe {
                Rule {
                    name: r.Name().map(|s| s.to_string()).unwrap_or_default(),
                    program: r.ApplicationName().map(|s| expand(&s.to_string())).unwrap_or_default(),
                    inbound: r.Direction().is_ok_and(|d| d == NET_FW_RULE_DIR_IN),
                    enabled: r.Enabled().is_ok_and(|e| e.as_bool()),
                    allow: r.Action().is_ok_and(|a| a == NET_FW_ACTION_ALLOW),
                    ports: r.LocalPorts().map(|s| s.to_string()).unwrap_or_default(),
                }
            };
            out.push((r, read));
        }
        Ok(out)
    }

    /// How many kinds of network (domain, private, public) the firewall is on for.
    fn profiles_on(p: &INetFwPolicy2) -> u32 {
        [NET_FW_PROFILE2_DOMAIN, NET_FW_PROFILE2_PRIVATE, NET_FW_PROFILE2_PUBLIC].into_iter().filter(|k| unsafe { p.get_FirewallEnabled(*k) }.map_or(true, |on| on.as_bool())).count() as u32
    }

    pub fn check(exe: &str, port: u16) -> windows::core::Result<bool> {
        let _com = Com::new();
        let p = policy()?;
        let list = unsafe { p.Rules()? };
        let read: Vec<Rule> = rules(&list)?.into_iter().map(|(_, r)| r).collect();
        Ok(open(profiles_on(&p), &read, exe, port))
    }

    /// With the administrator's rights: the rules blocking the program switched off, and the one
    /// rule letting the port in on every kind of network, in place of any of that name before.
    pub fn set_right(exe: &str, port: u16) -> windows::core::Result<()> {
        let _com = Com::new();
        let p = policy()?;
        let list = unsafe { p.Rules()? };
        let all = rules(&list)?;
        for (r, _) in all.iter().filter(|(_, x)| blocks(x, exe)) {
            unsafe { r.SetEnabled(VARIANT_FALSE)? };
        }
        let name = BSTR::from(RULE);
        // (Remove takes one rule of the name at a time)
        for _ in 0..all.iter().filter(|(_, x)| x.name == RULE).count() {
            unsafe { list.Remove(&name)? };
        }
        let rule: INetFwRule = unsafe { CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER)? };
        unsafe {
            rule.SetName(&name)?;
            rule.SetDescription(&BSTR::from(ABOUT))?;
            rule.SetApplicationName(&BSTR::from(exe))?;
            // (the protocol before the ports: the firewall takes ports only for TCP and UDP)
            rule.SetProtocol(NET_FW_IP_PROTOCOL_TCP.0)?;
            rule.SetLocalPorts(&BSTR::from(port.to_string()))?;
            rule.SetDirection(NET_FW_RULE_DIR_IN)?;
            rule.SetAction(NET_FW_ACTION_ALLOW)?;
            rule.SetProfiles(NET_FW_PROFILE2_ALL.0)?;
            rule.SetEnabled(VARIANT_TRUE)?;
            list.Add(&rule)?;
        }
        Ok(())
    }

    /// The program `exe` started again with the administrator's rights to set the firewall right
    /// for `port` (Windows asks the player), waited for.
    pub fn elevated(exe: &str, port: u16) -> Result<(), bool> {
        let file = HSTRING::from(exe);
        let params = HSTRING::from(format!("{ARG} {port}"));
        let verb = HSTRING::from("runas");
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: SW_HIDE.0,
            ..Default::default()
        };
        if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
            return Err(e.code() == ERROR_CANCELLED.to_hresult());
        }
        if info.hProcess.is_invalid() {
            return Err(false);
        }
        let mut code = 1u32;
        unsafe {
            WaitForSingleObject(info.hProcess, INFINITE);
            let got = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
            got.map_err(|_| false)?;
        }
        if code == 0 {
            Ok(())
        } else {
            Err(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(name: &str, program: &str, allow: bool, ports: &str) -> Rule {
        Rule { name: name.into(), program: program.into(), inbound: true, enabled: true, allow, ports: ports.into() }
    }

    const EXE: &str = r"C:\Users\O'Brien\AppData\Local\Omsi-Hub\omsi-hub.exe";

    #[test]
    fn the_phones_get_in_only_with_our_rule_for_the_port_and_nothing_blocking() {
        let ours = rule(RULE, EXE, true, "47811");
        assert!(open(3, std::slice::from_ref(&ours), EXE, 47811));
        assert!(!open(3, &[], EXE, 47811));
        assert!(!open(3, &[rule(RULE, EXE, true, "50000")], EXE, 47811));
        // (Windows' own rule from its question, clicked away: it blocks the program)
        let block = rule("omsi-hub.exe", &EXE.to_uppercase().replace('\\', "/"), false, "*");
        assert!(!open(3, &[ours.clone(), block.clone()], EXE, 47811));
        // a blocking rule switched off, outbound, or another program's does not count
        for b in [Rule { enabled: false, ..block.clone() }, Rule { inbound: false, ..block.clone() }, rule("x", r"C:\Other\x.exe", false, "*")] {
            assert!(open(3, &[ours.clone(), b], EXE, 47811));
        }
        // our rule switched off, or blocking, does not let them in
        assert!(!open(3, &[Rule { enabled: false, ..ours.clone() }], EXE, 47811));
        assert!(!open(3, &[Rule { allow: false, ..ours.clone() }], EXE, 47811));
        // the firewall off on every kind of network: nothing to set right
        assert!(open(0, &[block], EXE, 47811));
    }

    #[test]
    fn a_rules_ports_as_the_firewall_writes_them() {
        for (ports, yes) in [("47811", true), ("*", true), ("Any", true), ("80, 47811", true), ("47000-48000", true), ("50000", false), ("48000-49000", false), ("", false), ("x", false)] {
            assert_eq!(takes_port(ports, 47811), yes, "{ports}");
        }
    }

    #[test]
    fn started_to_set_the_firewall_by_its_argument_and_a_port() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(port_of_args(&args(&["omsi-hub.exe"])), None);
        assert_eq!(port_of_args(&args(&["omsi-hub.exe", ARG, "47811"])), Some(Some(47811)));
        assert_eq!(port_of_args(&args(&["omsi-hub.exe", ARG, "80"])), Some(None));
        assert_eq!(port_of_args(&args(&["omsi-hub.exe", ARG])), Some(None));
        assert!(same_program("c:/users/o'brien/appdata/local/omsi-hub/OMSI-HUB.EXE", EXE));
        assert!(!same_program("", EXE));
    }
}

#[cfg(all(test, windows))]
mod on_windows {
    /// Windows' own firewall read, as the app reads it when the phone & tablet is on: it answers
    /// (the 1.2.0 app crashed here as it started).
    #[test]
    fn the_firewall_of_this_computer_is_read() {
        let exe = std::env::current_exe().unwrap().to_string_lossy().into_owned();
        assert!(super::check(&exe, 47811).is_some());
    }
}
