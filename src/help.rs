pub fn show(command: Option<&str>) -> Result<(), String> {
    let text=match command{
        None=>"DEVAGATCHI — offline developer pets\n\nUSAGE: devagatchi COMMAND [--global|--repo|--both]\n\nCOMMANDS\n  init NAME [PATH] [MODE]  Adopt a pet; default scope: global, mode: cozy\n  connect                  Connect current repo to global pet; baseline old history\n  status                   Show both applicable pets; no arguments does the same\n  feed LINES               Manually feed positive lines (shares daily food budget)\n  mode cozy|traditional    Change decay rate, preserving elapsed care\n  pet cat|dog|bird|lizard   Change appearance\n  color COLOR              cyan, green, yellow, blue, magenta, red, white, off\n  exclude PATH             Exclude a file/directory prefix from future meals\n  help [COMMAND]           Command-specific help\n\nOPTIONS\n  --global  User-wide pet; --repo  Current checkout pet; --both  Both pets\n  -h, --help  Help; -V, --version  Version\n\nIf both pets exist, choose a scope when feeding or changing settings.\nGlobal state: $XDG_DATA_HOME/devagatchi/pet.txt or ~/.local/share/devagatchi/pet.txt\nRepository state: .devagatchi.local (automatically locally Git-ignored)",
        Some("init")=>"USAGE: devagatchi init NAME [PATH] [cozy|traditional] [--global|--repo]\nDefault: global, current directory, cozy. Existing pets are never overwritten.\nRun twice with different scopes to adopt two pets. Existing commits are the baseline.",
        Some("connect")=>"USAGE: devagatchi connect\nConnect the current checkout to your global pet. Existing history earns no food.",
        Some("status")=>"USAGE: devagatchi status [--global|--repo|--both]\nShows food and mood. Default: both applicable pets. Does not feed or scan history.",
        Some("feed")=>"USAGE: devagatchi feed LINES [--global|--repo|--both]\nExample: devagatchi feed 25 --both. Manual lines use the same daily formula as commits.",
        Some("mode")=>"USAGE: devagatchi mode cozy|traditional [--global|--repo|--both]\nOld decay applies up to the change. Switching mode does not refill food.",
        Some("pet")=>"USAGE: devagatchi pet cat|dog|bird|lizard [--global|--repo|--both]",
        Some("color")=>"USAGE: devagatchi color cyan|green|yellow|blue|magenta|red|white|off [SCOPE]\nNonempty NO_COLOR, TERM=dumb, and redirected output disable ANSI colors.",
        Some("exclude")=>"USAGE: devagatchi exclude PATH [SCOPE]\nExclude a repository-relative file or directory prefix. No globs. Applies only to future work.",
        Some("help")=>"USAGE: devagatchi help [COMMAND]",
        Some(_)=>return Err("Unknown help command".into()),
    };
    println!("{text}\n\n{}", crate::care::GUIDE);
    Ok(())
}
