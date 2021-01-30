# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('* 명령어, 무림별호, 도움말, 무공, 표현, 도우미, 메인설정, 스크립트 중에 선택하세요')
        elif line == '명령어':
            init_commands()
            ob.sendLine('* 명령어가 업데이트 되었습니다.')
        elif line == '무림별호':
            NICKNAME.load()
            ob.sendLine('* 무림별호가 업데이트 되었습니다.')
        elif line == '도움말':
            HELP.load()
            ob.sendLine('* 도움말이 업데이트 되었습니다.')
        elif line == '무공':
            MUGONG.load()
            ob.sendLine('* 무공이 업데이트 되었습니다.')
        elif line == '표현':
            EMOTION.load()
            ob.sendLine('* 표현이 업데이트 되었습니다.')
        elif line == '도우미':
            DOUMI.load()
            ob.sendLine('* 도우미가 업데이트 되었습니다.')
        elif line == '메인설정':
            MAIN_CONFIG.load()
            ob.sendLine('* 메인설정이 업데이트 되었습니다.')
        elif line == '스크립트':
            SCRIPT.load()
            ob.sendLine('* 스크립트가 업데이트 되었습니다.')
