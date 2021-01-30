# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [기연이름] 기연삭제')
            return
            
        msg = ''
        if line not in ONEITEM.index:
            ob.sendLine('☞ 그런 아이템은 없습니다.!')
            return
        index = ONEITEM.index[line]
        ONEITEM.attr.__delitem__(index)
        ONEITEM.save()
        ob.sendLine('☞ 기연이 삭제되었습니다.')

