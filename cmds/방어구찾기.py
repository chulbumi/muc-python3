# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        c = 0
        for index in Item.Items:
            item = Item.Items[index]
            if item['종류'] == '방어구':
                c += 1
                ob.sendLine('%s : %s'% (item.getNameA(), item.index))
        if c == 0:
            ob.sendLine('☞ 찾으시는 아이템이 없습니다.')
        

