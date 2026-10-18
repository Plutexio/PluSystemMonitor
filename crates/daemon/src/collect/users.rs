use std::collections::HashMap;
use std::fs;

/// uid → user name from /etc/passwd, re-read when an unknown uid shows up.
#[derive(Default)]
pub struct Users {
    names: HashMap<u32, String>,
}

impl Users {
    pub fn name(&mut self, uid: u32) -> String {
        if !self.names.contains_key(&uid) {
            self.reload();
        }
        // Remember misses too (e.g. LDAP users), so we don't re-read every time.
        self.names
            .entry(uid)
            .or_insert_with(|| uid.to_string())
            .clone()
    }

    fn reload(&mut self) {
        let Ok(passwd) = fs::read_to_string("/etc/passwd") else {
            return;
        };
        self.names.extend(parse_passwd(&passwd));
    }
}

fn parse_passwd(passwd: &str) -> impl Iterator<Item = (u32, String)> + '_ {
    passwd.lines().filter_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let uid = fields.nth(1)?.parse().ok()?;
        Some((uid, name.to_owned()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_passwd() {
        let entries: Vec<_> = parse_passwd(
            "root:x:0:0::/root:/bin/bash\n# comment\nplutex:x:1000:1000::/home/p:/bin/zsh\n",
        )
        .collect();
        assert_eq!(
            entries,
            [(0, "root".to_owned()), (1000, "plutex".to_owned())]
        );
    }
}
