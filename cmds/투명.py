# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        ob.sendLine('%d' % time.time())
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        
        if ob['투명상태'] == 0:
            ob['투명상태'] = 1
            ob.sendLine('☞ 투명상태가 되었습니다')
        else:
            ob['투명상태'] = 0
            ob.sendLine('☞ 투명상태가 해제되었습니다')
        """
        from client import Client
        for ply in Client.players:
            if ply.state != ACTIVE:
                continue
        """
        """
        Player.CFG = ['자동습득', '비교거부', '접촉거부', '동행거부', '전음거부', 
    '외침거부', '방파말거부', '간략설명', '엘피출력', '나침반제거',
    '운영자안시거부', '사용자안시거부', '입출입메세지거부', 
    '타인전투출력거부', '자동무공시전', '순위거부', '수련모드', '잡담시간보기']
        
        """

