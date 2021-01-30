# -*- coding: utf-8 -*-

import os


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

def ttoNumber(s):
    if len(s) > 1 and s[0] == '0':
        return s
    try:
        return int(s)
    except ValueError:
        try:
            return float(s)
        except ValueError:
            return s


def toNumber(s):
    try:
        v = float(s)
        if s.find('.') == -1:
            return int(s)
        return v
    except ValueError:
        return s
#        try:
#            return int(s)
#        except ValueError:
#            return s

def load_script(path):
    try:
        f = open(path)
    except IOError:
        #print 'load_script(%s) IOError' % path
        return None
    object = {}
    """
    [segment_name]
    #key_name
    :data
    ;comment
    """
    status = -1
    segName = ''

    for line in f:

        line = line.strip()
        if len(line) == 0:
            continue
        
        #comment = line.find('；')
        comment = find(line, '；')
        if comment == 0:
            continue
        elif comment != -1:
            line = line[0:comment]
        line = line.strip()
        
        if line[0] == '[':
            segname = line[1:-1].strip()
            if segname not in object:
                segment = {}
                object[segname] = segment
            else:
                if type(object[segname]) == dict:
                    seglist = []
                    segment = {}
                    seg = object[segname]
                    seglist.append(seg)
                    seglist.append(segment)
                    object[segname] = seglist
                else:
                    segment = {}
                    object[segname].append(segment)

        elif line[0] == '#':
            keyname = line[1:]

            if type(object[segname]) is dict:
#if object[segname].has_key(keyname):
#                    print '%s dup! %s' % (keyname, path)
                object[segname][keyname] = ''
            else:
#                if object[segname][-1].has_key(keyname):
#                    print '%s dup!! %s' % (keyname, path)
                object[segname][-1][keyname] = ''
        elif line[0] == ':':
            keydata = line[1:]
            keydata = toNumber(keydata)
            if type(object[segname]) is dict:
                if object[segname][keyname] == '':
                    if keydata == '':
                        keydata = ' '
                    object[segname][keyname] = keydata
                else:
                    object[segname][keyname] = str(object[segname][keyname]) + '\r\n' + str(keydata)
            else:
                if object[segname][-1][keyname] == '':
                    if keydata == '':
                        keydata = ' '
                    object[segname][-1][keyname] = keydata
                else:
                    object[segname][-1][keyname] = str(object[segname][-1][keyname]) + '\r\n' + str(keydata)
        else:
            continue
            
    f.close()
    return object

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

def save_script(f, x):
    """
    [segment_name]
    #key_name
    :data
    ;comment
    """
    if type(x) is not dict:
        return False
    if type(f) is not file:
        return False
        
    for segName in x:
        if type(x[segName]) != list:
            f.write('[' + str(segName) + ']\n\n')
            for keyName in x[segName]:
                f.write('#' + str(keyName) + '\n')
                
                if type(x[segName][keyName]) == list:
                    for keyData in x[segName][keyName]:
                        f.write(':' + str(keyData) + '\n')
                else:
                    if type(x[segName][keyName]) == int or type(x[segName][keyName]) == long:
                        f.write(':' + str(x[segName][keyName]) + '\n')
                    else:
                        lines = x[segName][keyName]
                        for line in lines:
                            #f.write(':' + str(x[segName][keyName]) + '\n')
                            f.write(':' + line + '\n')
                f.write('\n')
            f.seek(-2, os.SEEK_CUR)
            f.write('\n；\n')
        else:
            seglist = x[segName]
            for segment in seglist:
                f.write('[' + str(segName) + ']\n\n')
                for keyName in segment:
                    f.write('#' + str(keyName) + '\n')
                    
                    if type(segment[keyName]) == list:
                        for keyData in segment[keyName]:
                            f.write(':' + str(keyData) + '\n')
                    else:
                        f.write(':' + str(segment[keyName]) + '\n')
                    f.write('\n')
                f.seek(-2, os.SEEK_CUR)
                f.write('\n；\n')


def save_object(f, x):
    if type(x) is not dict:
        return False
    if type(f) is not file:
        return False
    f.write('# -*- coding: utf-8 -*-\n\n')
    f.write('obj = ')
    save_dict(f, x, 0)


def load_object(path):
    try:
        execfile(path)
    except:
        print('ERROR : execfile() in load_object(' + path + ')')
        return None

    try:
        o = locals()['obj']
    except:
        print('ERROR : locals()[] in load_object(' + path + ')')
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

def toUni(s):
    return unicode(s.decode('euc-kr'))

def toJson(x):
    obj = {}
    if type(x) is not dict:
        return False
        
    for segName in x:
        seg = toUni(segName)
        obj[seg] = {}
        if type(x[segName]) != list:
            for keyName in x[segName]:
                key = toUni(keyName)
                
                if type(x[segName][keyName]) == list:
                    obj[seg][key] = []
                    for keyData in x[segName][keyName]:
                        data = toUni(keyData)
                        obj[seg][key].append(data)
                else:
                    if type(x[segName][keyName]) == int or type(x[segName][keyName]) == long or type(x[segName][keyName]) == float:
                        obj[seg][key] = x[segName][keyName]
                    else:
                        obj[seg][key] = []
                        lines = x[segName][keyName]
                        if len(lines) == 1:
                             obj[seg][key] = toUni(x[segName][keyName])
                        else:
                            obj[seg][key] = []
                            for line in lines:
                                l = toUni(line)
                                obj[seg][key].append(l)
        else:
            seglist = x[segName]
            for segment in seglist:
                seg = toUni(segment)
                obj[seg] = {}
                for keyName in segment:
                    key = toUni(keyName)
                    if type(segment[keyName]) == list:
                        obj[seg][key] = []
                        for keyData in segment[keyName]:
                            data = toUni(keyData)
                            obj[seg][key].append(data)
                    else:
                        data = toUni(segment[keyName])
                        obj[seg][key] = data
    return obj

import json
import io
obj = load_script("data/mob/낙양성/82.mob");
o2 = toJson(obj)
#print(o2)
with io.open("murim.json", "w", encoding='utf8') as json_file:
    data = json.dumps(o2, indent=4, sort_keys=True, ensure_ascii=False)
    json_file.write(unicode(data))

