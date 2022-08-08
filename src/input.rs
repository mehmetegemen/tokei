use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, str::FromStr};

use tokei::{Language, LanguageType, Languages};

type LanguageMap = BTreeMap<LanguageType, Language>;

#[derive(Deserialize, Serialize, Debug)]
struct Output {
    #[serde(flatten)]
    languages: LanguageMap,
    #[serde(rename = "Total")]
    totals: Language,
}

macro_rules! supported_formats {
    ($(
        ($name:ident, $feature:expr, $variant:ident [$($krate:ident),+]) =>
            $parse_kode:expr,
            $print_kode:expr,
    )+) => (
        $( // for each format
            $( // for each required krate
                #[cfg(feature = $feature)] extern crate $krate;
            )+
        )+

        /// Supported serialization formats.
        ///
        /// To enable all formats compile with the `all` feature.
        #[cfg_attr(test, derive(strum_macros::EnumIter))]
        #[derive(Debug)]
        pub enum Format {
            Json,
            $(
                #[cfg(feature = $feature)] $variant
            ),+
            // TODO: Allow adding format at runtime when used as a lib?
        }

        impl Format {
            pub fn supported() -> &'static [&'static str] {
                &[
                    "json",
                    $(
                        #[cfg(feature = $feature)] stringify!($name)
                    ),+
                ]
            }

            pub fn all() -> &'static [&'static str] {
                &[
                    $( stringify!($name) ),+
                ]
            }

            pub fn all_feature_names() -> &'static [&'static str] {
                &[
                    $( $feature ),+
                ]
            }

            pub fn not_supported() -> &'static [&'static str] {
                &[
                    $(
                        #[cfg(not(feature = $feature))] stringify!($name)
                    ),+
                ]
            }

            pub fn parse(input: &str) -> Option<LanguageMap> {
                if input.is_empty() {
                    return None
                }

                if let Ok(Output { languages, .. }) = serde_json::from_str::<Output>(input) {
                    return Some(languages);
                }

                $(
                    // attributes are not yet allowed on `if` expressions
                    #[cfg(feature = $feature)]
                    {
                        let parse = &{ $parse_kode };

                        if let Ok(Output { languages, .. }) = parse(input) {
                            return Some(languages)
                        }
                    }
                )+

                // Didn't match any of the compiled serialization formats
                None
            }

            pub fn print(&self, languages: &Languages) -> Result<String, Box<dyn Error>> {
                let output = Output {
                    languages: (*languages).to_owned(),
                    totals: languages.total()
                };

                match *self {
                    Format::Json => Ok(serde_json::to_string(&output)?),
                    $(
                        #[cfg(feature = $feature)] Format::$variant => {
                            let print= &{ $print_kode };
                            Ok(print(&output)?)
                        }
                    ),+
                }
            }
        }

        impl FromStr for Format {
            type Err = String;

            fn from_str(format: &str) -> Result<Self, Self::Err> {
                match format {
                    "json" => Ok(Format::Json),
                    $(
                        stringify!($name) => {
                            #[cfg(feature = $feature)]
                            return Ok(Format::$variant);

                            #[cfg(not(feature = $feature))]
                            return Err(format!(
"This version of tokei was compiled without \
any '{format}' serialization support, to enable serialization, \
reinstall tokei with the features flag.

    cargo install tokei --features {feature}

If you want to enable all supported serialization formats, you can use the 'all' feature.

    cargo install tokei --features all\n",
                                format = stringify!($name),
                                feature = $feature)
                            );
                        }
                    ),+
                    format => Err(format!("{:?} is not a supported serialization format", format)),
                }
            }
        }
    )
}

// The ordering of these determines the attempted order when parsing.
supported_formats!(
    (cbor, "cbor", Cbor [serde_cbor, hex]) =>
        |input| {
            hex::FromHex::from_hex(input)
                .map_err(|e: hex::FromHexError| <Box<dyn Error>>::from(e))
                .and_then(|hex: Vec<_>| Ok(serde_cbor::from_slice(&hex)?))
        },
        |languages| serde_cbor::to_vec(&languages).map(hex::encode),

    (json, "json", Json [serde_json]) =>
        serde_json::from_str,
        serde_json::to_string,

    (yaml, "yaml", Yaml [serde_yaml]) =>
        serde_yaml::from_str,
        serde_yaml::to_string,
);

pub fn add_input(input: &str, languages: &mut Languages) -> bool {
    use std::fs::File;
    use std::io::Read;

    let map = match File::open(input) {
        Ok(mut file) => {
            let contents = {
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .expect("Couldn't read file");
                contents
            };

            convert_input(&contents)
        }
        Err(_) => {
            if input == "stdin" {
                let mut stdin = ::std::io::stdin();
                let mut buffer = String::new();

                let _ = stdin.read_to_string(&mut buffer);
                convert_input(&buffer)
            } else {
                convert_input(input)
            }
        }
    };

    if let Some(map) = map {
        *languages += map;
        true
    } else {
        false
    }
}

fn convert_input(contents: &str) -> Option<LanguageMap> {
    self::Format::parse(contents)
}

pub fn create_repo_dl_path(input: &[&str]) -> Result<Vec<(String, String)>, ()> {
    let git_repo_paths: Vec<_> = input
        .iter()
        .filter(|uri| is_git(uri))
        .map(|repo_url| {
            let v = repo_url
                .split("/")
                .collect::<Vec<&str>>()
                .into_iter()
                .rev()
                .take(2)
                .collect::<Vec<&str>>();

            let (repo_name, user) = (
                v[0].trim_end_matches(".git"),
                v[1].trim_start_matches("git@github.com:"),
            );

            let new_repo_dir = format!("/tmp/tokei/{repo_name}__{user}");
            new_repo_dir
        })
        .zip(
            input
                .iter()
                .map(|uri| uri.to_string())
                .collect::<Vec<String>>(),
        )
        .collect();

    if git_repo_paths.len() == 0 {
        return Err(());
    }

    git_repo_paths.iter().for_each(|(repo, _)| {
        std::fs::create_dir_all(std::path::Path::new(repo))
            .expect("Cannot create directory for downloading repo associated with {uri}");
    });

    Ok(git_repo_paths)
}

pub fn is_git(uri: &str) -> bool {
    // TODO: Check remote repo's validity with libgit
    uri.contains("git@")
        || uri.contains("https://github.com")
        || uri.contains("https://www.github.com")
}

#[cfg(test)]
mod tests {
    use super::*;

    use strum::IntoEnumIterator;
    use tokei::Config;

    use std::path::Path;

    mod git_repo_paths {
        use super::create_repo_dl_path;
        use std::fs::remove_dir_all;
        use std::path::Path;

        #[test]
        fn single_url_wo_www_is_successful() {
            // Create a single https input url without www
            let input = vec!["https://github.com/user/repo"];

            // Create dirs and get dir paths
            let dir = create_repo_dl_path(&input).unwrap();

            // Check if url to temproray directory conversion is correct
            assert_eq!(dir[0].0, "/tmp/tokei/repo__user");
            assert_eq!(dir[0].1, input[0]);

            // Check if the directory is created
            assert!(Path::new(&dir[0].0).is_dir());
        }

        #[test]
        fn multiple_urls_is_successful() {
            // Create a single https input url without www
            let input = vec![
                "https://github.com/user/repo",
                "https://www.github.com/another_user/repo",
            ];

            // Create dirs and get dir paths
            let dir = create_repo_dl_path(&input).unwrap();

            // Check if url to temproray directory conversion is correct
            assert_eq!(dir[0].0, "/tmp/tokei/repo__user");
            assert_eq!(dir[0].1, input[0]);
            assert_eq!(dir[1].0, "/tmp/tokei/repo__another_user");
            assert_eq!(dir[1].1, input[1]);
        }

        #[test]
        fn multiple_urls_and_local_dir_is_successful() {
            // Create a single https input url without www
            let input = vec![
                "https://github.com/user/repo",
                "https://www.github.com/another_user/repo",
                "~/project",
                ".",
                "..",
                "./dir",
                "/dir/dir",
            ];

            // Create dirs and get dir paths
            let dir = create_repo_dl_path(&input).unwrap();

            // Check if url to temproray directory conversion is correct
            assert_eq!(dir.len(), 2);
        }

        #[test]
        fn no_input_errors() {
            // Create a single https input url without www
            let input = vec![];

            // Create dirs and get dir paths
            let dir = create_repo_dl_path(&input);

            assert!(dir.is_err());
        }

        #[test]
        fn ssh_is_successful() {
            // Create a single https input url without www
            let input = vec!["git@github.com:user/repo.git"];

            // Create dirs and get dir paths
            let dir = create_repo_dl_path(&input).unwrap();

            // Check if url to temproray directory conversion is correct
            assert_eq!(dir[0].0, "/tmp/tokei/repo__user");
            assert_eq!(dir[0].1, input[0]);
        }

        fn teardown() {
            if Path::new("/tmp/tokei").is_dir() {
                remove_dir_all("/tmp/tokei").expect("cannot teardown git input tests");
            }
        }
    }

    #[test]
    fn formatting_print_matches_parse() {
        // Get language results from sample dir
        let data_dir = Path::new("tests").join("data");
        let mut langs = Languages::new();
        langs.get_statistics(&[data_dir], &[], &Config::default());

        // Check that the value matches after serializing and deserializing
        for variant in Format::iter() {
            let serialized = variant
                .print(&langs)
                .expect(&format!("Failed serializing variant: {:?}", variant));
            let deserialized = Format::parse(&serialized)
                .expect(&format!("Failed deserializing variant: {:?}", variant));
            assert_eq!(*langs, deserialized);
        }
    }
}
