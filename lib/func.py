# -*- coding: utf-8 -*-

def fillSpace(ref, out):
    l = len(ref.encode('euc-kr'))
    s = len(out.encode('euc-kr'))
    if s < l:
        return out + ' ' * (l - s)
    return out 

def log(line):
    import time
    print(time.strftime('[%Y-%m-%d %H:%M:%S] ', time.localtime()) + line)

def getStrCnt(line):
    from random import randint
    tok = line.split()
    cnt = 1
    l = len(tok)
    if l == 3:
        cnt = getInt(tok[2])
    elif l > 3:
        return tok[randint(1, l - 2)], getInt(tok[-1])
    return tok[1], cnt

def stripANSI(line):
    #buf = line.strip()
    buf = line
    if buf == '':
        return line
    found = False
    ret = ''
    for i in range(0, len(buf)):
        if ord(buf[i]) == 155:
            continue
        if ord(buf[i]) == 8: 
            if len(ret) != 0:
                ret = ret[:-1]
            continue
        if ord(buf[i]) == 27:
            found = True
            continue
        if found == True and buf[i] == 'm':
            found = False
            continue
        if found == False:
            ret += buf[i]
    return ret
    
def getInt(s):
    i = 0
    if s == '':
        return 0
    try:
        i = int(s)
    except:
        if s[0].isdigit() == False:
            return 0
        c = 0
        for c in range(len(s)):
            if s[c].isdigit() == False and c != 0:
                return int( s[0:c] )
    return i

def getNameOrder(name):
    order = getInt(name)
    if order != 0:
        for i in range( len(name) ):
            if name[i].isdigit() == False:
                name = name[i:]
                break
    else:
        order = 1
    return name, order

def loadScriptFile(path):
    try:
        #f = open('data/script/' + path)
        with open('data/script/' + path) as fp:
            return json.load(fp)

    except IOError:
        return None
    #return f.readlines() 
