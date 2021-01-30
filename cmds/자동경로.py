# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            if len(ob.autoMoveList) == 0:
                ob.sendLine('☞ 사용법: [지정한줄임말] 자동경로')
                return
            else:
                ob.autoMoveList = []
                ob.sendLine('☞ 자동경로가 삭제되었습니다.')
                return

            msg = '['
            for path in ob.autoMoveList:
                msg += '%s, ' % path
            msg = msg[:-2] + ']'
            ob.write(msg)
            return
        
        if line not in ob.alias:
            ob.sendLine('☞ 해당 줄임말이 없어요. ^^')
            return
        ob.autoMoveList = ob.alias[line].split(';') 
        ob.sendLine('☞ 자동경로를 설정하였어요. ^^')
        next = ob.autoMoveList.pop(0)
        ob.do_command(next)
        

