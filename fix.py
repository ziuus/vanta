import os

def replace(file, old, new):
    content = open(file).read()
    open(file, 'w').write(content.replace(old, new))

replace('src/widgets/cmatrix.rs',
'''const CHARSET: &str = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜｦﾝ0123456789";''',
'''const CHARSET: &str = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜｦﾝ0123456789";

#[allow(clippy::needless_range_loop)]
#[allow(clippy::unnecessary_cast)]''')

replace('src/widgets/cmatrix.rs',
'''    let mut rows: Vec<Vec<(char, u8)>> = vec![vec![(' ', 0); width]; height];''',
'''    let mut rows: Vec<Vec<(char, u8)>> = vec![vec![(' ', 0); width]; height];
    let matrix_chars: Vec<char> = CHARSET.chars().collect();''')

replace('src/widgets/cmatrix.rs',
'''                let ch_idx = (seed.wrapping_mul(13).wrapping_add(row.wrapping_mul(31)))
                    % CHARSET.len();
                let ch = CHARSET.as_bytes()[ch_idx % CHARSET.len()] as char;''',
'''                let ch_idx = (seed.wrapping_mul(13).wrapping_add(row.wrapping_mul(31)))
                    % matrix_chars.len();
                let ch = matrix_chars[ch_idx];''')

replace('src/monitors/cpu.rs', 'fs::read_dir(&entry.path())', 'fs::read_dir(entry.path())')
replace('src/monitors/cpu.rs', '(core_count + 1) / 2', 'core_count.div_ceil(2)')
replace('src/monitors/disk.rs', 'or_insert_with(Vec::new)', 'or_default()')
replace('src/screens/overview.rs', "fn section_header<'a>(f: &mut Frame, area: Rect, label: &str, theme: &app::Theme) {", "fn section_header(f: &mut Frame, area: Rect, label: &str, theme: &app::Theme) {")
replace('src/screens/overview.rs', '&format!(" {} ", label)', 'format!(" {} ", label)')
replace('src/widgets/calendar.rs', 'now.year() as i32 * 12', 'now.year() * 12')
replace('src/widgets/music_viz.rs', '(*silence_frames + 1).min(u32::MAX)', '(*silence_frames + 1)')

content = open('src/monitors/processes.rs').read()
content = content.replace("pub fn render(", "#[allow(clippy::too_many_arguments)]\npub fn render(")
open('src/monitors/processes.rs', 'w').write(content)

