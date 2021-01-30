# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [아이템 이름] 먹어')
            return
        if ob.act == ACT_REST:
            ob.sendLine('☞ 먹을 수 있는 상황이 아니네요. ^_^')
            return
        name, order = getNameOrder(line)
        item = ob.findObjInven(name, order)
        if item == None:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if item['옵션'] != None:
            ob.sendLine(item['옵션'])
            del item['아이템속성']
            del item['옵션']
            return
