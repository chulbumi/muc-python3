# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 체인지')
            return
        ply = ob.env.findObjName(line)
        if ply == None or is_player(ply) == False:
            ob.sendLine('여기에 없나봐요^^')
            return
            
        c = ply.channel
        p = ply.channel.player
        
        ply.channel = ob.channel
        ply.channel.player = ob.channel.player
        
        ob.channel = c
        ob.channel.player = p
        
        ply.INTERACTIVE = 1
        ob.INTERACTIVE = 1
        
        ply.input_to(ob.parse_command)
        ob.input_to(ply.parse_command)
        
        ob.sendLine('\r\n%s%s 몸을 교환합니다.' % (ply['이름'], han_wa(ply['이름'])))
        ply.sendLine('\r\n%s%s 몸을 교환합니다.' % (ob['이름'], han_wa(ob['이름'])))
