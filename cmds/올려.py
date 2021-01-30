# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려')
            return

        l = ['힘', '민첩성', '맷집', '명중', '회피', '필살', '운', '내공', '체력']
        l1 = ['명중', '회피', '필살', '운']
        if line not in l:
            ob.sendLine('☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 올려')
            return
     
        p = getInt(ob['특성치'])
        if p == 0:
            ob.sendLine('☞ 더이상 올릴 수 있는 여유 특성치가 없습니다.')
            return

        if line in l1:
            """
            all100 = False
            all200 = False
            if ob['명중'] >= 100 and ob['회피'] >= 100 and ob['필살'] >= 100 and ob['운'] >= 100:
                all100 = True
            if ob['명중'] >= 200 and ob['회피'] >= 200 and ob['필살'] >= 200 and ob['운'] >= 200:
                all200 = True
            if all100 == False:
                if getInt(ob[line]) >= 100:
                    ob.sendLine('☞ 더이상 올릴 수 없습니다.')
                    return
            if all200 == False:
                if getInt(ob[line]) >= 200:
                    ob.sendLine('☞ 더이상 올릴 수 없습니다.')
                    return
            if getInt(ob[line]) >= 300:
                ob.sendLine('☞ 더이상 올릴 수 없습니다.')
                return
            """
            if getInt(ob[line]) >= 100:
                ob.sendLine('☞ 더이상 올릴 수 없습니다.')
                return
        if line == '민첩성':
            if ob['민첩성'] >= 2800:
                ob.sendLine('☞ 더이상 올릴 수 없습니다.')
                return

        x = ob[line+'특성치']
        if x == '':
            x = 0
            if line in l1:
                x = ob[line]
                if x == '':
                    x = 0
        x += 1
        ob[line+'특성치'] = x

        ob['특성치'] -= 1
        if ob[line] == '':
            ob[line] = 0
        if line == '내공':
            ob['최고내공'] += 10
        elif line == '체력':
            ob['최고체력'] += 100
        else:
            ob[line] += 1 

        ob.save()
        ob.sendLine('☞ [%s] 특성치를 올렸습니다.' % line)
