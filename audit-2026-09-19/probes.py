"""Run bounded differential probes against the supplied C AWK and rawk."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
CASES = [
    ('exit_end', 'BEGIN { exit 7 } END { print "END" }', ''),
    ('short_circuit', 'BEGIN { x=0; print (0 && ++x), x; print (1 || ++x), x }', ''),
    ('record_counter', '{$0=$0; print NR,FNR} END {print NR}', 'a\nb\n'),
    ('fs_regex', 'BEGIN { FS="[,:]+" } {print NF,$1,$2,$3}', 'a,b:c\n'),
    ('rs_dynamic', '{print $0; RS=":"}', 'a\nb:c:'),
    ('regex_longest', 'BEGIN {match("ab", /a|ab/); print RSTART,RLENGTH}', ''),
    ('regex_invalid', 'BEGIN {r="["; print ("abc" ~ r)}', ''),
    ('array_parameter', 'function f(a) {a[1]=9} BEGIN {x[1]=1; f(x); print x[1]}', ''),
    ('return_while', 'function f(i) {while(i++ < 2) {return 7} return 9} BEGIN {print f(0)}', ''),
    ('next_function', 'function f() {next} {f(); print "BAD"} END {print NR}', 'a\nb\n'),
    ('getline_file', '{getline x; print $0,x,NR,FNR}', 'a\nb\nc\nd\n', 'file'),
    ('getline_counter', '{getline x; print $0,x,NR,FNR}', 'a\nb\nc\nd\n'),
    ('getline_missing', 'BEGIN {print (getline x < "/nonexistent/rawk-audit-absent")}', ''),
    ('split_empty_clear', 'BEGIN {a[9]="old"; print split("a,,b",a,","); print a[2],(9 in a)}', ''),
    ('gsub_count', 'BEGIN {s="aaa"; print gsub(/a/,"b",s),s; print sub(/b/,"b",s)}', ''),
    ('numeric_string_compare', 'BEGIN { print ("01" == "1"), ("10" < "2") }', ''),
    ('numeric_prefix', 'BEGIN {print "12x"+0}', ''),
    ('field_increment', '{$1++; print $1,$0}', '3\n'),
    ('escaped_quote', 'BEGIN {print "a\\\"b"}', ''),
    ('range_pattern', 'NR==1,NR==2 {print}', 'a\nb\nc\n'),
    ('assignment_expr', 'BEGIN {print (x=3)}', ''),
    ('leading_decimal', 'BEGIN {print .5}', ''),
    ('underscore_identifier', 'BEGIN {_x=3; print _x}', ''),
    ('power_precedence', 'BEGIN {print -2^2}', ''),
    ('arity_panic', 'BEGIN {print substr("a")}', ''),
    ('program_file_cli', '{print FILENAME,$0}', 'a\nb\n', 'programfile'),
    ('argv_assignment', '{print x,$0}', 'a\n', 'assignment'),
]

def main():
    results = []
    env = dict(os.environ, LC_ALL='C')
    with tempfile.TemporaryDirectory(prefix='rawk-audit-') as work:
        for case in CASES:
            name, program, data, *mode = case
            args = [program]
            stdin = data.encode()
            path = Path(work) / 'input.txt'
            path.write_text(data)
            if mode == ['file']:
                args += [str(path)]
                stdin = b''
            elif mode == ['programfile']:
                prog = Path(work) / 'program.awk'
                prog.write_text(program)
                args = ['-f', str(prog), str(path)]
                stdin = b''
            elif mode == ['assignment']:
                args += ['x=7']
            row = {'name': name, 'program': program, 'input': data, 'mode': mode}
            for label, binary in [('c', ROOT/'c_awk/a.out'), ('rust', ROOT/'rawk/target/release/rawk')]:
                try:
                    out = subprocess.run([str(binary), *args], input=stdin, capture_output=True,
                                         cwd=work, env=env, timeout=3)
                    row[label] = {'code': out.returncode, 'stdout': out.stdout.decode(errors='backslashreplace'),
                                  'stderr': out.stderr.decode(errors='backslashreplace')}
                except subprocess.TimeoutExpired:
                    row[label] = {'timeout': True}
            row['same_stdout_and_status'] = all(row['c'].get(k) == row['rust'].get(k) for k in ('code','stdout','timeout'))
            results.append(row)
    output = Path(__file__).with_name('probes.json')
    output.write_text(json.dumps(results, indent=2, ensure_ascii=False)+'\n')
    for row in results:
        print(row['name'], 'C:', repr(row['c']), 'Rust:', repr(row['rust']))
    print('Differences:', sum(not r['same_stdout_and_status'] for r in results), '/', len(results))

if __name__ == '__main__':
    main()
