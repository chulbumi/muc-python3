# -*- coding: utf-8 -*-

import os
import json

def find(line, word):
    i = 0
    l = len(word)
    L = len(line)
    if l > L :
        return -1
    cnt = L - l
    while i < cnt:
        if line[i:i+l] == word:
            return i
        a = ord(line[i])
        if a >= 0xB0 and a <= 0xC8:
            i += 1
        i += 1
    return -1

def toNumber1(s):
    if s.find('.') == -1 and s.startswith('0'):
        return s
    try:
        v = float(s)
        if s.find('.') == -1:
            return int(s)
        return v
    except ValueError:
        return s

def toNumber(s):
    if s.find('.') == -1 and s.startswith('0') and len(s) > 1:
        return s
    try:
        v = float(s)
        if s.find('.') == -1:
            return int(s)
        return v
    except ValueError:
        return s

def toNumber0(s):
    if len(s) > 1 and s[0] == '0':
        return s
    try:
        return int(s)
    except ValueError:
        try:
            return float(s)
        except ValueError:
            return s

def load_script(path):
    try:
        with open(path) as fp:
            obj = json.load(fp)
        return obj
    except IOError:
        # print 'load_script(%s) IOError' % path
        return None

def save_list(f, x, first = 0):
    f.write('[\n')
    for l in x:
        if first != 0:
            for i in range(first):
                f.write('\t')
        if type(l) == int:
            f.write(str(l))
        elif type(l) == long:
            f.write(str(l))
        elif type(l) == float:
            f.write(str(l))
        elif type(l) == str:
            f.write('\'' + str(l) + '\'')
        elif type(l) == list:
            save_list(f, l, first)
        elif type(l) == dict:
            save_dict(f, l, first)
        if l is not x[-1]:
            f.write(',\n')
        else:
            f.write('\n')
    if first != 0:
        for i in range(first - 1):
            f.write('\t')
    f.write(']')


def save_dict(f, x, first = 0):
    f.write('{\n')
    for key in x:
        if first != 0:
            for i in range(first):
                f.write('\t')
        strk = str(key)
        if type(key) is str:
            strk = '\'' + str(key) + '\''

        if type(key) == str and key[0] == '_':
            continue
        if type(x[key]) == int or type(x[key]) == float or type(x[key]) == long:
            f.write(strk + ': ' + str(x[key]))
        elif type(x[key]) == str:
            """print (strk + ': \'' + str(x[key]) + '\'' + '\n')"""
            f.write(strk + ': \'' + str(x[key]) + '\'')
        elif type(x[key]) == list:
            f.write(strk + ': ')
            save_list(f, x[key], first + 1)
        elif type(x[key]) == dict:
            f.write(strk + ': ')
            save_dict(f, x[key], first + 1)
        
        if key is not x.keys()[-1]:
            if type(x[key]) == dict:
                f.write(',\n\n')
            else:
                f.write(',\n')
        else:
            if first != 0:
                f.write('\n')
    if first != 0:
        for i in range(first):
            f.write('\t')
    if first == 0:
        f.write('\n}')
    else:
        f.write('}')

def save_script(fp, x):
    """
    [segment_name]
    #key_name
    :data
    ;comment
    """
    if type(x) is not dict:
        return False
    json.dump(x, fp, sort_keys=True, ensure_ascii=False, indent=4)

def save_object(f, x):
    if type(x) is not dict:
        return False
    if type(f) is not file:
        return False
    f.write('# -*- coding: euc-kr -*-\n\n')
    f.write('obj = ')
    save_dict(f, x, 0)


def load_object(path):
    try:
        execfile(path)
    except:
        print 'ERROR : execfile() in load_object(' + path + ')'
        return None

    try:
        o = locals()['obj']
    except:
        print 'ERROR : locals()[] in load_object(' + path + ')'
        return None

    return o

"""
o = load_script('용파리')


f = open('m.py', 'w')
save_object(f, o)
f.close()
#print o
f = open('z.py', 'w')
save_script(f, o)
f.close()

#f = open('murim.cfg', 'U')
#for line in f:
#    print(line)
#f.close()
#load_object('m.py')
"""
