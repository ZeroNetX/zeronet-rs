use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, Clone, Default)]
pub enum Permission {
    PathProvider(String),
    #[default]
    None,
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D>(deserializer: D) -> Result<Permission, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        Ok(Permission::from(s.as_str()))
    }
}

impl Serialize for Permission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = match self {
            Permission::PathProvider(version) => format!("path_provider@{version}"),
            Permission::None => "".into(),
        };
        serializer.serialize_str(&string)
    }
}

impl From<&str> for Permission {
    fn from(s: &str) -> Self {
        let mut splited = s.split('@');
        let s = splited.next().unwrap();
        let version = splited.next().unwrap_or("0.0.1");
        match s {
            "path_provider" => Permission::PathProvider(version.into()),
            _ => Permission::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_from_str() {
        let permission = "path_provider";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::PathProvider("0.0.1".into()));

        let permission = "path_provider@0.0.2";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::PathProvider("0.0.2".into()));

        let permission = "path_provider#0.0.2";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::None);

        let permission = "";
        let perm: Permission = permission.into();
        assert_eq!(perm, Permission::None);
    }
}
