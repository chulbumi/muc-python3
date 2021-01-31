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
    return tossi.pick(word, '이(가)')

def han_ira(word):
    return tossi.pick(word, '이라')

def han_obj(word):
    return tossi.pick(word, '을(를)')

def han_un(word):
    return tossi.pick(word, '은(는)')

def han_wa(word):
    return tossi.pick(word, '과(와)')

def han_uro(word):
    return tossi.pick(word, '(으)로')


def han_i(word):
    return tossi.pick(word, '이')


def han_aya(word):
    return tossi.pick(word, '야')


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
