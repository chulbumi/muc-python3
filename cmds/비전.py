# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        from objs.skill import MUGONG
        
        if line == '':
            if ob['비전설정'] == '':
                ob.sendLine('☞ 비전 : 없음')
                return
            else:
                ob.sendLine('☞ 비전 : [[1;37m%s[0;37m]' % ob['비전설정'])
                return
        s = None
        vision = ob['비전이름']
        if line not in vision:
            ob.sendLine('☞ 당신은 그런 비전을 배운적이 없습니다.')
            return
        ob['비전설정'] = line
        ob.sendLine('☞ 비전을 지정하였습니다.')
