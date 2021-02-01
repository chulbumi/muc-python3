import tossi
from lib.func import stripANSI


def is_han(word):
    if len(word) <= 0:
        return False
    # UNICODE RANGE OF KOREAN: 0xAC00 ~ 0xD7A3
    for c in range(len(word)):
        if word[c] < "\uac00" or word[c] > "\ud7a3":
            return False
    return True


def han_iga(word):
    obj = tossi.parse('이(가)')
    return obj[word]

def han_ira(word):
    obj = tossi.parse('이라')
    return obj[word]

def han_obj(word):
    obj = tossi.parse('을(를)')
    return obj[word]

def han_un(word):
    obj = tossi.parse('은')
    return obj[word]

def han_wa(word):
    obj = tossi.parse('과(와)')
    return obj[word]

def han_uro(word):
    obj = tossi.parse('(으)로')
    return obj[word]

def han_i(word):
    obj = tossi.parse('이')
    return obj[word]

def han_aya(word):
    obj = tossi.parse('야')
    return obj[word]


def postPosition(line, name):
    s = line.find('(')
    if s == -1:
        return line
    e = line.find(')')
    pps = line[s:e + 1]
    sep = pps.find('/')
    pp1 = pps[1:sep]
    pp = tossi.pick(name, f"{pp1}")
    return line.replace(pps, pp)


def postPosition1(line):
    s = line.find('(')
    if s == -1:
        return line
    e = line.find(')')
    pps = line[s:e + 1]
    sep = pps.find('/')
    pp1 = pps[1:sep]
    pp = tossi.pick(stripANSI(line[:s]), f"{pp1}")
    return line.replace(pps, pp, 1)
