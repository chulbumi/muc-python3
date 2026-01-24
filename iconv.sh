find . -name "*.box" -type f -exec iconv -f euc-kr -t utf-8 "{}" -o "{}" \;
