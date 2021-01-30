# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        msg = '[단일아이템인덱스]\r\n'
        for index in Item.Items:
            item = Item.Items[index]
            if item.isOneItem():
                msg += '#%s\r\n:%s\r\n\r\n' % (item['이름'], item.index)
        ob.sendLine(msg)
