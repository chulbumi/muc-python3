# -*- coding: utf-8 -*-

def tell_room(env, line, ob = None):
    if len(line) == 0:
        return

    from lib.object import is_player
    for obj in env.objs:
        if is_player(obj) and obj is not ob:
            obj.sendLine(line)

def broadcast(line, ob = None):
    if len(line) == 0:
        return

    from objs.soul import Soul

    if ob == None:
        for c in Soul.clients:
            c.sendLine(line)
    else:
        for c in Soul.clients:
            if c != ob:
                c.sendLine(line)

def say(ob, line, *args):
    if len(line) == 0:
        return
    for c in ob.channel.clients:
        if c == ob:
            c.sendLine('you say : ' + line)
        else:
            c.sendLine(ob.get('이름') + ' say : ' + line) 
