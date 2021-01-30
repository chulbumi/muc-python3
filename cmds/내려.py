# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려')
            return

        l = ['힘', '민첩성', '맷집', '명중', '회피', '필살', '운', '내공', '체력']
        l1 = ['명중', '회피', '필살', '운']
        if line not in l:
            ob.sendLine('☞ 사용법: [힘|민첩성|맷집|명중|회피|필살|운|내공|체력] 내려')
            return

        x = ob[line + '특성치']
        if x == '':
            x = 0
            if x == 0 and line in l1:
                x = ob[line]
                if x == '':
                    x = 0

        if x <= 0:
            ob.sendLine('☞ [%s] 더이상 내릴 수 없습니다.' % line)
            return

        x -= 1
        ob[line + '특성치'] = x

        if ob['특성치'] == '':
            ob['특성치'] = 0
        ob['특성치'] += 1

        if line == '내공':
            ob['최고내공'] -= 10
        elif line == '체력':
            ob['최고체력'] -= 100
        else:
            ob[line] -= 1 

        ob.save()
        ob.sendLine('☞ [%s] 특성치를 내렸습니다.' % line)
