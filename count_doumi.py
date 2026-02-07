import json
with open('data/config/doumi.json') as f:
    data = json.load(f)
    story = data['도우미메인설정']['초기도우미']
    print(f'Python Total entries: {len(story)}')
    key_input = sum(1 for s in story if '키입력' in s)
    output_start = sum(1 for s in story if '출력시작' in s)
    output_end = sum(1 for s in story if '출력끝' in s)
    print(f'Key input markers: {key_input}')
    print(f'Output start markers: {output_start}')
    print(f'Output end markers: {output_end}')
