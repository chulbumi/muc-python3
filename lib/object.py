# -*- coding: utf-8 -*-

from objs.object import Object
from objs.world import World
from include.path import *
    
def is_item(obj):
    from objs.item import Item
    return isinstance(obj, Item)

def is_room(obj):
    from objs.room import Room
    return isinstance(obj, Room)
    
def is_player(obj):
    from objs.player import Player
    return isinstance(obj, Player)

def is_mob(obj):
    from objs.mob import Mob 
    return isinstance(obj, Mob)

def is_body(obj):
    from objs.body import Body
    return isinstance(obj, Body)
    
def create_object(path):
    try:
        exec(compile(open(path + '.py', "rb").read(), path + '.py', 'exec'))
    except IOError:
        print('create_object() IoError ' + path)
        return None

    try:
        obj = locals()['Obj']
    except KeyError:
        print('create_object() KeyError')
        return None

    o = obj()
    o.set('path', path)
    #o.create()
    #o.init()
    return o

# 낙양성/도구점
def get_room(ZoneRoom):

    i = ZoneRoom.find('/')
    if i == -1:
        return None

    zone_name = ZoneRoom[:i]
    room_name = ZoneRoom[i+1:]

    try:
        zone = World.zones[zone_name]
    except KeyError:
        zone = {}
        World.zones[zone_name] = zone
        
    try:
        room = zone[room_name]
    except KeyError:
        room = create_object(ZONE_PATH + ZoneRoom)
        if room == None:
            return None
        import time
        room.last_reset_time = time.time()
        zone[room_name] = room

    return room

def find_obj(env, objName, cnt = 1):
    count = 1
    for obj in env.objs:
        name = obj.get('이름')
        if name == objName or (is_player(obj) == False and name.find(objName) == 0):
            if cnt == count:
                return obj
            else:
                count += 1

    return None


def find_objN(env, line):
    objName = line.split(' ')[0]
    n = 1
    if len(line.split(' ')) >= 2:
        n = int(line.split(' ')[-1])
    return find_obj(env, objName, n)
    

def count_object(env, objName):
    count = 0
    for obj in env.objs:
        if obj.get('이름') == objName:
            count += 1
    return count

def item_count(objs):
    lst = {}
    for obj in objs:
        if is_item(obj):
            name = obj.get('이름')
            if name in lst:
                lst[name][1] += 1
            else:
                lst[name] = {}
                lst[name][0] = obj.get('이름')
                lst[name][1] = 1
    return lst


def item_countA(objs):
    lst = {}
    for obj in objs:
        if is_item(obj):
            name = obj.get('이름')
            if name in lst:
                lst[name][1] += 1
            else:
                lst[name] = {}
                lst[name][0] = obj.getA('이름')
                lst[name][1] = 1
                lst[name][2] = obj.get('설명1')
    return lst
    
def mob_count(objs):
    lst = {}
    for obj in objs:
        if is_mob(obj):
            name = obj.getA('이름')
            if name in lst:
                lst[name] += 1
            else:
                lst[name] = 1
    return lst

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

def load_script(path):
    try:
        f = open(path, 'U')
    except IOError:
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
            line = line[0:comment].strip()
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
                object[segname][keyname] = ''
            else:
                object[segname][-1][keyname] = ''
        elif line[0] == ':':
            keydata = line[1:]
            keydata = keydata.replace('\'', '\\\'')

            if type(object[segname]) is dict:
                if object[segname][keyname] == '':
                    object[segname][keyname] = keydata
                else:
                    if type(object[segname][keyname]) is not list:
                        keydatalist = [object[segname][keyname]]
                        object[segname][keyname] = keydatalist
                    object[segname][keyname].append(keydata)
            else:
                if object[segname][-1][keyname] == '':
                    object[segname][-1][keyname] = keydata
                else:
                    if type(object[segname][-1][keyname]) is not list:
                        keydatalist = [object[segname][-1][keyname]]
                        object[segname][-1][keyname] = keydatalist
                    object[segname][-1][keyname].append(keydata)
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
        if type(x[key]) == int:
            f.write(strk + ': ' + str(x[key]))
        if type(x[key]) == float:
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
        
        if key is not list(x.keys())[-1]:
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
                    f.write(':' + str(x[segName][keyName]) + '\n')
                f.write('\n')
            f.seek(-2, os.SEEK_CUR)
            #f.write('；──────────────────────────────────────\n')
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
                #f.write('；──────────────────────────────────────\n')


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
        exec(compile(open(path, "rb").read(), path, 'exec'))
    except:
        print('ERROR : execfile() in load_object(' + path + ')')
        return None

    try:
        o = locals()['obj']
    except:
        print('ERROR : locals()[] in load_object(' + path + ')')')'
        return None

    return o
    
def LoadObjectData(o, path):

    attr = load_script(path)
    if attr == None:
        return False

    o.attr = attr
    o.path = path

    return True

