# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if len(line) == 0:
            ob.sendLine('사용법: [아이템 인덱스] 아이템삭제')
            return

        item = getItem(line)
        if item == None:
            ob.sendLine('존재하지않는 아이템입니다.')
            return
        Item.Items.__delitem__(item.index)
        del item
        ob.sendLine('아이템이 삭제되었습니다.')
        

