# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['비전설정'] == '':
            ob.sendLine('☞ 지정된 비전이 없습니다.')
            return
        ob['비전설정'] = ''
        ob.sendLine('☞ 지정된 비전을 삭제합니다.')
